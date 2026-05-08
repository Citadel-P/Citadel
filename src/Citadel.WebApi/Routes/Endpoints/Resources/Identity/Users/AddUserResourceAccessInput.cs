using Application.Features.Identity.Users.Commands;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record AddUserResourceAccessInput(
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions)
{
    internal AddUserResourceAccess ToCommand(Guid userId) => new(userId, ResourceType, ResourceId, PermissionLevel, SpecificPermissions);
}
