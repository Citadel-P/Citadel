using Application.Features.Identity.Roles.Commands;
using Domain.Contracts.Resources.Role;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record RoleInput(string Name, IEnumerable<PermissionInput> Permissions)
{
    internal CreateRole ToCommand() => new(Name, Permissions.Select(x => x.ToModel()));
}

public sealed record PermissionInput(
    ResourceType ResourceType,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions)
{
    internal PatchPermissionModel ToModel() => new(ResourceType, PermissionLevel, SpecificPermissions);
}
