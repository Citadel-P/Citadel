using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Role;
using Hosting.Common;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Roles.Queries;

public sealed record GetAllRoles() : IQuery<Result<IEnumerable<RoleDetails>>>;

internal sealed class GetAllRolesHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAllRoles, Result<IEnumerable<RoleDetails>>>
{
    public async ValueTask<Result<IEnumerable<RoleDetails>>> Handle(GetAllRoles query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var roles = user is not null && !user.IsAdmin()
            ? await unitOfWork.Roles.GetAuthorizedAsync(user.GetUserId(), ResourceType.Role, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.Roles.GetAllAsync(cancellationToken);

        return Result.Success(roles.Select(role => new RoleDetails(role.Id, role.Name, role.RoleType, role.Permissions)));
    }
}
