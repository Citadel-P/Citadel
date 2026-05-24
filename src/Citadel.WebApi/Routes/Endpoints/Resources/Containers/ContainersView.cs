using Application.Permissions;
using Domain.Entities;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainersView(IEnumerable<ContainerView> Containers)
{
    internal static ContainersView Map(IEnumerable<Container> containersInfo)
        => new(ContainerView.Map(containersInfo));

    internal static async Task<ContainersView> Map(IEnumerable<Container> containers, IPermissionEvaluator permissionEvaluator)
    {
        var list = containers as Container[] ?? [.. containers];

        if (list.Length == 0)
            return new ContainersView([]);

        // All containers belong to the same platform capability scope.
        // Resolve once instead of N times.
        var platformId = list[0].PlatformId;

        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);

        var capabilities = CapabilityMapper.ToPlatformCapabilities(meta == default ? PermissionMetadata.Empty : meta);

        var views = new ContainerView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            views[i] = ContainerView.Map(list[i]) with
            {
                Capabilities = capabilities
            };
        }

        return new ContainersView(views);
    }
}
