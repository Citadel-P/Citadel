using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, ResourceAction.Update)]
public sealed record PatchTeam(Guid Id, JsonMergePatchDocument<PatchTeamModel> Patch) : ICommand<Result<TeamDetails>>
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
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.Action))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class PatchTeamHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(PatchTeam command, CancellationToken cancellationToken)
    {
        var team = await unitOfWork.Teams.GetDetailsAsync(command.Id, cancellationToken);
        if (team is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided team does not exist"));

        var actor = await unitOfWork.Actors.GetById(team.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<TeamDetails>(new NotFoundError("The provided actor does not exist"));

        var currentUserIds = (await unitOfWork.Teams.GetUserIdsAsync(team.Id, cancellationToken)).ToArray();
        var currentRoleIds = (await unitOfWork.Roles.GetActorRoleIdsAsync(team.ActorId, cancellationToken)).ToArray();
        var current = new PatchTeamModel(team.IsEnabled, currentUserIds, currentRoleIds, null);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchTeamModel);

        if (patched.UserIds is not null)
        {
            var userIds = patched.UserIds.Distinct().ToArray();
            if (userIds.Length > 0)
            {
                var users = await unitOfWork.Users.GetAllAsync(userIds, cancellationToken) ?? [];
                var existingUserIds = users.Select(x => x.Id).ToHashSet();
                var missingUserId = userIds.FirstOrDefault(x => !existingUserIds.Contains(x));
                if (missingUserId != Guid.Empty)
                    return Result.Failure<TeamDetails>(new NotFoundError($"User with ID {missingUserId} does not exist"));
            }

            await unitOfWork.Teams.ReplaceMembersAsync(team.Id, userIds, cancellationToken);
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
                    return Result.Failure<TeamDetails>(new NotFoundError($"Role with ID {missingRoleId} does not exist"));
            }

            await unitOfWork.Roles.ReplaceActorRolesAsync(team.ActorId, roleIds, cancellationToken);
        }

        if (patched.ResourceAccesses is not null)
        {
            var resourceAccesses = patched.ResourceAccesses
                .Distinct()
                .Select(x => ResourceAccess.Create(x.ResourceType, x.ResourceId, team.ActorId, x.Action))
                .ToArray();

            await unitOfWork.ResourceAccesses.ReplaceAsync(team.ActorId, resourceAccesses, cancellationToken);
        }

        if (patched.IsEnabled.HasValue && patched.IsEnabled.Value != actor.IsEnabled)
        {
            var setEnabledResult = actor.SetEnabled(patched.IsEnabled.Value);
            if (setEnabledResult.IsFailure(out var error))
                return Result.Failure<TeamDetails>(error);
        }

        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new TeamDetails(team.Id, team.Name, team.ActorId, actor.IsEnabled, team.TotalMembers, team.Roles);
    }
}
