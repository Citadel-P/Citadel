using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Application.Services.Identity;

namespace Application.Features.Identity.Teams.Commands;

[RequirePermission(ResourceType.Team, PermissionLevel.Write)]
public sealed record CreateTeam(
    string Name,
    IEnumerable<Guid>? UserIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<TeamResourceAccessModel>? ResourceAccesses = null) : ICommand<Result<TeamDetails>>
{
    internal sealed class Validator : AbstractValidator<CreateTeam>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleForEach(x => x.UserIds).NotEmpty();
            RuleForEach(x => x.RoleIds).NotEmpty();
            RuleForEach(x => x.ResourceAccesses)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class CreateTeamHandler(IUnitOfWork unitOfWork, IActorScopeEvictor evictor) : ICommandHandler<CreateTeam, Result<TeamDetails>>
{
    public async ValueTask<Result<TeamDetails>> Handle(CreateTeam command, CancellationToken cancellationToken)
    {
        var conflicts = await unitOfWork.Teams.GetConflictsAsync(command.Name, null, cancellationToken);
        if (conflicts)
            return Result.Failure<TeamDetails>(new ConflictError("Name already exists"));

        var actor = Actor.Create(ActorType.Team, new ActorMetadata(command.Name));
        var team = Team.Create(command.Name, actor.Id);

        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.Teams.AddAsync(team, cancellationToken);

        var userIds = command.UserIds?.Distinct().ToArray() ?? [];
        var roleIds = command.RoleIds?.Distinct().ToArray() ?? [];
        var resourceAccesses = command.ResourceAccesses?.Distinct().ToArray() ?? [];

        if (userIds.Length > 0)
        {
            var users = await unitOfWork.Users.GetAllAsync(userIds, cancellationToken) ?? [];
            var existingUserIds = users.Select(x => x.Id).ToHashSet();
            var missingUserId = userIds.FirstOrDefault(x => !existingUserIds.Contains(x));
            if (missingUserId != Guid.Empty)
                return Result.Failure<TeamDetails>(new NotFoundError($"User with ID {missingUserId} does not exist"));

            await unitOfWork.Teams.ReplaceMembersAsync(team.Id, userIds, cancellationToken);
            await evictor.EvictUsers(userIds, cancellationToken);
        }

        if (roleIds.Length > 0)
        {
            var roles = await unitOfWork.Roles.GetAllAsync(roleIds, cancellationToken) ?? [];
            var existingRoleIds = roles.Select(x => x.Id).ToHashSet();
            var missingRoleId = roleIds.FirstOrDefault(x => !existingRoleIds.Contains(x));
            if (missingRoleId != Guid.Empty)
                return Result.Failure<TeamDetails>(new NotFoundError($"Role with ID {missingRoleId} does not exist"));

            await unitOfWork.Roles.ReplaceActorRolesAsync(team.ActorId, roleIds, cancellationToken);
            await evictor.EvictPermissionsForActorAsync(team.ActorId, cancellationToken);
        }

        if (resourceAccesses.Length > 0)
        {
            var accessRows = resourceAccesses
                .Select(x => ResourceAccess.Create(x.ResourceType, x.ResourceId, team.ActorId, x.PermissionLevel, x.SpecificPermissions))
                .ToArray();

            await unitOfWork.ResourceAccesses.ReplaceAsync(team.ActorId, accessRows, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        return new TeamDetails(team.Id, team.Name, team.ActorId, actor.IsEnabled);
    }
}
