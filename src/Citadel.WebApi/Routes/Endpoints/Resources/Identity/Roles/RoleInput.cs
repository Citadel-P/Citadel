using Application.Features.Identity.Roles.Commands;
using Domain.Entities.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record RoleInput(string Name, IEnumerable<PermissionInput> Permissions)
{
    internal CreateRole ToCommand() => new(Name, Permissions.Select(x => x.ToDomain(Guid.Empty)));
}

public sealed record PermissionInput(Hosting.Common.ResourceType ResourceType, Hosting.Common.ResourceAction ResourceAction)
{
    internal Permission ToDomain(Guid roleId) => Permission.Create(roleId, ResourceType, ResourceAction);
}
