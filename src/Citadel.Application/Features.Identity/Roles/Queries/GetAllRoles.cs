using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Roles.Queries;

[RequirePermission(ResourceType.Role, PermissionLevel.Read)]
public sealed record GetAllRoles() : IQuery<Result<IEnumerable<RoleDetails>>>;

internal sealed class GetAllRolesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllRoles, Result<IEnumerable<RoleDetails>>>
{
    public async ValueTask<Result<IEnumerable<RoleDetails>>> Handle(GetAllRoles query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var roles = user is not null && !user.IsAdmin
            ? await unitOfWork.Roles.GetAuthorizedAsync(user.UserId, ResourceType.Role, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.Roles.GetAllAsync(cancellationToken);

        return Result.Success(roles.Select(role => new RoleDetails(role.Id, role.Name, role.RoleType, role.Permissions)));
    }
}
