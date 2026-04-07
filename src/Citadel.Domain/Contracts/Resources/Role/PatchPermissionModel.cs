using Domain.Entities.Identity;
using Hosting.Common;

namespace Domain.Contracts.Resources.Role;

public sealed record PatchRolePermissionsModel(IEnumerable<PatchPermissionModel> Permissions);

public sealed record PatchPermissionModel(ResourceType ResourceType, ResourceAction ResourceAction)
{
    public Permission ToDomain(Guid roleId) => Permission.Create(roleId, ResourceType, ResourceAction);
}
