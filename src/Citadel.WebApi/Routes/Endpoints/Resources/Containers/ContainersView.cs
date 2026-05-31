using Application.Permissions;
using Domain.Entities;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainersView(IEnumerable<ContainerView> Containers, PlatformCapabilities Capabilities)
{
    internal static ContainersView Map(IEnumerable<Container> containers) =>
        new(containers.Select(ContainerView.Map), PlatformCapabilities.Empty);

    internal static async Task<ContainersView> Map(IEnumerable<Container> containers, Guid platformId, IPermissionEvaluator permissionEvaluator)
    {
        var list = containers as Container[] ?? [.. containers];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        if (list.Length == 0)
            return new ContainersView([], CapabilityMapper.ToPlatformCapabilities(resourcesPerms));

        var capabilities = CapabilityMapper.ToPlatformCapabilities(resourcesPerms);

        var views = new ContainerView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            views[i] = ContainerView.Map(list[i]) with
            {
                Capabilities = capabilities
            };
        }

        return new ContainersView(views, capabilities);
    }
}
