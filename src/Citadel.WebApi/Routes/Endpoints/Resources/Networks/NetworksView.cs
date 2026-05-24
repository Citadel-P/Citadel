using Application.Permissions;
using Domain.Contracts.Resources.Networks;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record NetworksView(IEnumerable<DockerNetworkResultView> Networks)
{
    internal static async Task<NetworksView> Map(IEnumerable<DockerNetworkResult> networks, IPermissionEvaluator permissionEvaluator)
    {
        var list = networks as DockerNetworkResult[] ?? [.. networks];

        if (list.Length == 0)
            return new NetworksView([]);

        // All networks belong to the same platform capability scope. Resolve once instead of N times.
        var platformId = list[0].PlatformId;

        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);

        var capabilities = CapabilityMapper.ToNetworkCapabilities(meta == default ? PermissionMetadata.Empty : meta);

        var views = new DockerNetworkResultView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            views[i] = DockerNetworkResultView.Map(list[i]) with
            {
                Capabilities = capabilities
            };
        }

        return new NetworksView(views);
    }
}
