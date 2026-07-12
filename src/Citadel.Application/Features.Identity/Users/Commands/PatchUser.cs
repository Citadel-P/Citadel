using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Domain;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Application.Services.Identity;
using Application.Services.Licensing;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write)]
public sealed record PatchUser(Guid Id, JsonMergePatchDocument<PatchUserModel> Patch) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : PatchCommandValidator<PatchUser, PatchUserModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchUserModel,
                modelValidator: new PatchUserModelValidator())
        {
        }
    }

    internal sealed class PatchUserModelValidator : AbstractValidator<PatchUserModel>
    {
        public PatchUserModelValidator()
        {
            When(x => x.Email is not null, () => RuleFor(x => x.Email!).EmailAddress());
            When(x => x.Password is not null, () => RuleFor(x => x.Password!).MinimumLength(6).MaximumLength(128));

            RuleForEach(x => x.TeamIds).NotEmpty();
            RuleForEach(x => x.RoleIds).NotEmpty();

            RuleForEach(x => x.ResourceAccesses)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class PatchUserHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    ILicenseQuotaService licenseQuotaService) : ICommandHandler<PatchUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(PatchUser command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetUserUpdateStateAsync(command.Id, null, null, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var actor = await unitOfWork.Actors.GetById(state.User.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        var currentTeamIds = (await unitOfWork.Users.GetTeamIdsAsync(state.User.Id, cancellationToken)).ToArray();
        var currentRoleIds = (await unitOfWork.Roles.GetActorRoleIdsAsync(state.User.ActorId, cancellationToken)).ToArray();
        var current = new PatchUserModel(state.User.Email, null, state.IsEnabled, currentTeamIds, currentRoleIds, null);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchUserModel);
        var emailChanged = patched.Email is not null && !string.Equals(patched.Email, state.User.Email, StringComparison.OrdinalIgnoreCase);

        if (emailChanged)
        {
            var conflicts = await unitOfWork.Users.GetConflictsAsync(state.User.Name, patched.Email!, state.User.Id, cancellationToken);
            if (conflicts.EmailExists)
                return Result.Failure<UserDetails>(new ConflictError("Email already exists"));
        }

        if (patched.TeamIds is not null)
        {
            var teamIds = patched.TeamIds.Distinct().ToArray();
            if (teamIds.Length > 0)
            {
                var teams = await unitOfWork.Teams.GetAllAsync(teamIds, cancellationToken) ?? [];
                var existingTeamIds = teams.Select(x => x.Id).ToHashSet();
                var missingTeamId = teamIds.FirstOrDefault(x => !existingTeamIds.Contains(x));
                if (missingTeamId != Guid.Empty)
                    return Result.Failure<UserDetails>(new NotFoundError($"Team with ID {missingTeamId} does not exist"));
            }

            await unitOfWork.Users.ReplaceTeamsAsync(state.User.Id, teamIds, cancellationToken);
            await evictor.EvictUsers(new[] { state.User.Id }, cancellationToken);
        }

        if (patched.RoleIds is not null)
        {
            var roleIds = patched.RoleIds.Distinct().ToArray();
            if (roleIds.Length > 0)
            {
                var roles = await unitOfWork.Roles.GetAllAsync(roleIds, cancellationToken) ?? [];
                var existingRoleIds = roles.Select(x => x.Id).ToHashSet();
                var missingRoleId = roleIds.FirstOrDefault(x => !existingRoleIds.Contains(x));
                if (missingRoleId != Guid.Empty)
                    return Result.Failure<UserDetails>(new NotFoundError($"Role with ID {missingRoleId} does not exist"));
            }

            await unitOfWork.Roles.ReplaceActorRolesAsync(state.User.ActorId, roleIds, cancellationToken);
            await evictor.EvictPermissionsForActorAsync(state.User.ActorId, cancellationToken);
        }

        if (patched.ResourceAccesses is not null)
        {
            var resourceAccesses = patched.ResourceAccesses
                .Distinct()
                .Select(x => ResourceAccess.Create(x.ResourceType, x.ResourceId, state.User.ActorId, x.PermissionLevel, x.SpecificPermissions))
                .ToArray();

            await unitOfWork.ResourceAccesses.ReplaceAsync(state.User.ActorId, resourceAccesses, cancellationToken);
        }

        state.User.UpdateMetadata(email: patched.Email);
        if (patched.Password is not null)
            state.User.SetPassword(patched.Password);

        if (patched.IsEnabled.HasValue && patched.IsEnabled.Value != actor.IsEnabled)
        {
            if (patched.IsEnabled.Value)
            {
                var quotaResult = await licenseQuotaService.EnsureCanIncreaseAsync(
                    new Dictionary<LicenseLimit, int>
                    {
                        [LicenseLimit.ActiveUsers] = 1
                    },
                    unitOfWork,
                    cancellationToken);
                if (quotaResult.IsFailure())
                    return Result.Failure<UserDetails>(quotaResult.Errors);
            }

            var setEnabledResult = actor.SetEnabled(patched.IsEnabled.Value);
            if (setEnabledResult.IsFailure(out var error))
                return Result.Failure<UserDetails>(error);
        }

        await unitOfWork.Users.UpdateAsync(state.User, cancellationToken);
        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var persistedResourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(state.User.ActorId, cancellationToken);
        return new UserDetails(
            state.User.Id,
            state.User.Name,
            state.User.Email,
            state.User.ActorId,
            actor.IsEnabled,
            state.User.CreatedAt,
            state.User.CreatedByActorId,
            ResourceAccesses: persistedResourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.ResourceName, x.PermissionLevel, x.SpecificPermissions)));
    }
}
