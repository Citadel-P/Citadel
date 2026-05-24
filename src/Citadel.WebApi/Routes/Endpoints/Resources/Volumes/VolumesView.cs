using Application.Permissions;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumesView(IEnumerable<DockerVolumeResultView> Volumes)
{
    internal static async Task<VolumesView> Map(IEnumerable<DockerVolumeResult> volumes, IPermissionEvaluator permissionEvaluator)
    {
        var list = volumes as DockerVolumeResult[] ?? [.. volumes];

        if (list.Length == 0)
            return new VolumesView([]);

        // All volumes belong to the same platform capability scope. Resolve once instead of N times.
        var platformId = list[0].PlatformId;

        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);

        var capabilities = CapabilityMapper.ToVolumeCapabilities(meta == default ? PermissionMetadata.Empty : meta);

        var views = new DockerVolumeResultView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            views[i] = DockerVolumeResultView.Map(list[i]) with
            {
                Capabilities = capabilities
            };
        }

        return new VolumesView(views);
    }
}
