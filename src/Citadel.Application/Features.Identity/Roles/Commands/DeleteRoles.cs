using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, PermissionLevel.Execute)]
public sealed record DeleteRoles(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<DeleteRoles>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class DeleteRolesHandler(
    IUnitOfWork unitOfWork,
    IActorScopeEvictor evictor,
    IUserContextAccessor userContext) : ICommandHandler<DeleteRoles, Result>
{
    public async ValueTask<Result> Handle(DeleteRoles command, CancellationToken cancellationToken)
    {
        var roles = await unitOfWork.Roles.GetAllAsync(command.Ids, cancellationToken);
        if (roles is null || !roles.Any())
            return Result.Failure(new NotFoundError("No roles found matching the provided IDs."));

        var roleArray = roles.ToArray();
        if (roleArray.Any(role => role.RoleType == Domain.RoleType.System))
            return Result.Failure(new ConflictError("System roles cannot be deleted."));

        var affectedActorIds = new HashSet<Guid>();
        foreach (var role in roleArray)
        {
            var actorIds = await unitOfWork.Roles.GetActorIdsByRoleIdAsync(role.Id, cancellationToken);
            foreach (var actorId in actorIds)
            {
                var actorIdsForPrincipal = await unitOfWork.Teams.GetAffectedPrincipalActorIdsAsync(actorId, cancellationToken);
                affectedActorIds.UnionWith(actorIdsForPrincipal);
            }
        }

        await unitOfWork.Roles.RemoveRangeAsync(roleArray.Select(static role => role.Id), cancellationToken);
        foreach (var role in roleArray)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    role.Id,
                    role.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.RoleDeleted,
                    new RoleDeleted(IdentityActivity.Snapshot(role))),
                cancellationToken);
        }
        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictActors(affectedActorIds, cancellationToken);
        return Result.Success();
    }
}
