using Application.Services.Identity;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Write)]
public sealed record PatchTeam(Guid Id, JsonMergePatchDocument<PatchTeamModel> Patch) : ICommand<Result<TeamDetails>>, IAdministratorRequest
{
    internal sealed class Validator : PatchCommandValidator<PatchTeam, PatchTeamModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchTeamModel,
                modelValidator: new PatchTeamModelValidator())
        {
        }
    }

    internal sealed class PatchTeamModelValidator : AbstractValidator<PatchTeamModel>
    {
        public PatchTeamModelValidator()
        {
            RuleForEach(x => x.UserIds).NotEmpty();
            RuleForEach(x => x.RoleIds).NotEmpty();
            RuleForEach(x => x.ResourceAccesses)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class PatchTeamHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IAdministratorGuard administratorGuard,
    ILicenseEntitlementService entitlementService,
    IUserContextAccessor userContext) : ICommandHandler<PatchTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(PatchTeam command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.Id, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var oldSnapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, team.Id, cancellationToken);
        if (oldSnapshot is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var actor = await unitOfWork.Actors.GetById(team.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided actor does not exist"));

        var currentUserIds = (await unitOfWork.Teams.GetUserIdsAsync(team.Id, cancellationToken)).ToArray();
        var currentRoleIds = (await unitOfWork.Roles.GetActorRoleIdsAsync(team.ActorId, cancellationToken)).ToArray();
        var current = new PatchTeamModel(team.IsEnabled, currentUserIds, currentRoleIds, null);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchTeamModel);
        var userIds = patched.UserIds?.Distinct().ToArray();
        var roleIds = patched.RoleIds?.Distinct().ToArray();
        var roles = Array.Empty<Role>();

        if (roleIds is { Length: > 0 })
        {
            roles = (await unitOfWork.Roles.GetAllAsync(roleIds, cancellationToken) ?? []).ToArray();
            var existingRoleIds = roles.Select(x => x.Id).ToHashSet();
            var missingRoleId = roleIds.FirstOrDefault(x => !existingRoleIds.Contains(x));
            if (missingRoleId != Guid.Empty)
                return Result.Failure<TeamDetails>(new NotFoundError($"Role with ID {missingRoleId} does not exist"));
        }

        if (userIds is not null)
        {
            var addsMembers = userIds.Except(currentUserIds).Any();
            if (userIds.Length > 0)
            {
                var users = await unitOfWork.Users.GetAllAsync(userIds, cancellationToken) ?? [];
                var existingUserIds = users.Select(x => x.Id).ToHashSet();
                var missingUserId = userIds.FirstOrDefault(x => !existingUserIds.Contains(x));
                if (missingUserId != Guid.Empty)
                    return Result.Failure<TeamDetails>(new NotFoundError($"User with ID {missingUserId} does not exist"));
            }

            if (addsMembers)
            {
                var resultingRoles = roles;
                if (roleIds is null && currentRoleIds.Length > 0)
                    resultingRoles = (await unitOfWork.Roles.GetAllAsync(currentRoleIds, cancellationToken) ?? []).ToArray();

                var hasResourceAccess = patched.ResourceAccesses is not null
                    ? patched.ResourceAccesses.Any()
                    : await unitOfWork.ResourceAccesses.ExistsForActorAsync(team.ActorId, cancellationToken);

                if (resultingRoles.Any(x => x.RoleType == RoleType.Custom) || hasResourceAccess)
                {
                    var entitlement = await entitlementService.EnsureEnabledAsync(
                        LicenseCapability.CustomAccessControl,
                        cancellationToken);
                    if (entitlement.IsFailure(out var entitlementError))
                        return Result.Failure<TeamDetails>(entitlementError);
                }
            }

            await unitOfWork.Teams.ReplaceMembersAsync(team.Id, userIds, cancellationToken);
        }

        if (roleIds is not null)
        {
            var addedRoleIds = roleIds.Except(currentRoleIds).ToHashSet();
            if (roles.Any(x => addedRoleIds.Contains(x.Id) && x.RoleType == RoleType.Custom))
            {
                var entitlement = await entitlementService.EnsureEnabledAsync(
                    LicenseCapability.CustomAccessControl,
                    cancellationToken);
                if (entitlement.IsFailure(out var entitlementError))
                    return Result.Failure<TeamDetails>(entitlementError);
            }

            await unitOfWork.Roles.ReplaceActorRolesAsync(team.ActorId, roleIds, cancellationToken);
        }

        if (patched.ResourceAccesses is not null)
        {
            var resourceAccesses = patched.ResourceAccesses
                .Distinct()
                .Select(x => ResourceAccess.Create(x.ResourceType, x.ResourceId, team.ActorId, x.PermissionLevel, x.SpecificPermissions))
                .ToArray();

            var currentResourceAccesses = (await unitOfWork.ResourceAccesses
                .GetAllByActorIdAsync(team.ActorId, cancellationToken))
                .Select(x => ResourceAccess.FromPersistence(
                    x.Id,
                    x.ResourceType,
                    x.ResourceId,
                    x.ActorId,
                    x.PermissionLevel,
                    x.SpecificPermissions));

            if (LicenseAccessControlPolicy.ExpandsResourceAccess(currentResourceAccesses, resourceAccesses))
            {
                var entitlement = await entitlementService.EnsureEnabledAsync(
                    LicenseCapability.CustomAccessControl,
                    cancellationToken);
                if (entitlement.IsFailure(out var entitlementError))
                    return Result.Failure<TeamDetails>(entitlementError);
            }

            await unitOfWork.ResourceAccesses.ReplaceAsync(team.ActorId, resourceAccesses, cancellationToken);
        }

        if (patched.IsEnabled.HasValue && patched.IsEnabled.Value != actor.IsEnabled)
        {
            var setEnabledResult = actor.SetEnabled(patched.IsEnabled.Value);
            if (setEnabledResult.IsFailure(out var error))
                return Result.Failure<TeamDetails>(error);
        }

        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);

        if (userIds is not null || roleIds is not null || patched.IsEnabled == false)
        {
            var guardResult = await administratorGuard.EnsureAdministratorRemainsAsync(cancellationToken);
            if (guardResult.IsFailure(out var guardError))
                return Result.Failure<TeamDetails>(guardError);
        }

        var newSnapshot = await IdentityActivity.CaptureTeamAsync(unitOfWork, team.Id, cancellationToken);
        if (newSnapshot is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        if (!IdentityActivity.Same(oldSnapshot, newSnapshot))
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    team.Id,
                    team.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.TeamUpdated,
                    new TeamUpdated(oldSnapshot, newSnapshot)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);
        if (patched.UserIds is not null
            || patched.RoleIds is not null
            || patched.ResourceAccesses is not null
            || patched.IsEnabled.HasValue)
        {
            await evictor.EvictUsers(
                currentUserIds.Union(userIds ?? currentUserIds),
                cancellationToken);
            await evictor.EvictPermissionsForActorAsync(team.ActorId, cancellationToken);
        }

        return new TeamDetails(
            team.Id,
            team.Name,
            team.ActorId,
            actor.IsEnabled,
            team.TotalMembers,
            Roles: team.Roles);
    }
}
