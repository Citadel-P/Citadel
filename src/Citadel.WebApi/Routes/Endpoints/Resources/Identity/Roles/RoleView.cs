using Domain.Contracts.Resources.Role;
using Domain.Entities.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record RoleView(Guid Id, string Name, Domain.RoleType RoleType, IEnumerable<PermissionView> Permissions)
{
    internal static RoleView Map(RoleDetails role) => new(role.Id, role.Name, role.RoleType, role.Permissions.Select(PermissionView.Map));
}

public sealed record PermissionView(Hosting.Common.ResourceType ResourceType, Hosting.Common.ResourceAction ResourceAction)
{
    internal static PermissionView Map(Permission permission) => new(permission.ResourceType, permission.ResourceAction);
}
