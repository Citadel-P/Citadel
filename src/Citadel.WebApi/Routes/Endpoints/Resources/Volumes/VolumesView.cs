using Application.Permissions;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumesView(IEnumerable<DockerVolumeResultView> Volumes, ResourceCapabilities Capabilities)
{
    internal static async Task<VolumesView> Map(IEnumerable<DockerVolumeResult> volumes, IPermissionEvaluator permissionEvaluator)
        => await Map(volumes, permissionEvaluator, null);

    internal static async Task<VolumesView> Map(
        IEnumerable<DockerVolumeResult> volumes,
        IPermissionEvaluator permissionEvaluator,
        IReadOnlyDictionary<VolumeBackupCoverageKey, BackupCoverageView>? coverage)
    {
        var list = volumes as DockerVolumeResult[] ?? [.. volumes];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Platform);
        if (list.Length == 0)
            return new VolumesView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        // All volumes belong to the same platform capability scope. Resolve once instead of N times.
        var platformId = list[0].PlatformId;

        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var contentMeta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Volume);

        var capabilities = CapabilityMapper.ToVolumeCapabilities(
            meta == default ? PermissionMetadata.Empty : meta,
            contentMeta == default ? PermissionMetadata.Empty : contentMeta);

        var views = new DockerVolumeResultView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var volume = list[i];
            BackupCoverageView? backupCoverage = null;
            coverage?.TryGetValue(new VolumeBackupCoverageKey(volume.PlatformId, volume.Name), out backupCoverage);
            views[i] = DockerVolumeResultView.Map(volume) with
            {
                BackupCoverage = backupCoverage,
                Capabilities = capabilities
            };
        }

        return new VolumesView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}
