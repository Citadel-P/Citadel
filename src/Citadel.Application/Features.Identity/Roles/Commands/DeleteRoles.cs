using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common;
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
    IActorScopeEvictor evictor) : ICommandHandler<DeleteRoles, Result>
{
    public async ValueTask<Result> Handle(DeleteRoles command, CancellationToken cancellationToken)
    {
        var roles = await unitOfWork.Roles.GetAllAsync(command.Ids, cancellationToken);
        if (roles is null || !roles.Any())
            return Result.Failure(new NotFoundError("No roles found matching the provided IDs."));

        var roleArray = roles.ToArray();
        if (roleArray.Any(role => role.RoleType == Domain.RoleType.System))
            return Result.Failure(new ConflictError("System roles cannot be deleted."));

        var affectedUserIds = new HashSet<Guid>();
        foreach (var role in roleArray)
        {
            var actorIds = await unitOfWork.Roles.GetActorIdsByRoleIdAsync(role.Id, cancellationToken);
            foreach (var actorId in actorIds)
            {
                var userIds = await unitOfWork.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);
                affectedUserIds.UnionWith(userIds);
            }
        }

        await unitOfWork.Roles.RemoveRangeAsync(roleArray.Select(static role => role.Id), cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictUsers(affectedUserIds, cancellationToken);
        return Result.Success();
    }
}
