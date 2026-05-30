using Application.Permissions;
using Domain.Contracts.Resources.Role;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record RolesView(IEnumerable<RoleView> Roles, ResourceCapabilities Capabilities)
{
    internal static async Task<RolesView> Map(IEnumerable<RoleDetails> roles, IPermissionEvaluator permissionEvaluator)
    {
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Role);
        if (!roles.Any())
            return new RolesView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var items = roles.Select(RoleView.Map);
        return new RolesView(items, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}
