using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Commands;

[RequirePermission(ResourceType.Role, PermissionLevel.Execute)]
public sealed record DeleteRoles(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteRolesHandler(IUnitOfWork unitOfWork) : ICommandHandler<DeleteRoles, Result>
{
    public async ValueTask<Result> Handle(DeleteRoles command, CancellationToken cancellationToken)
    {
        var roles = await unitOfWork.Roles.GetAllAsync(command.Ids, cancellationToken);
        if (roles is null || !roles.Any())
            return Result.Failure(new NotFoundError("No roles found matching the provided IDs."));

        if (roles.Any(role => role.RoleType == Domain.RoleType.System))
            return Result.Failure(new ConflictError("System roles cannot be deleted."));

        await unitOfWork.Roles.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
