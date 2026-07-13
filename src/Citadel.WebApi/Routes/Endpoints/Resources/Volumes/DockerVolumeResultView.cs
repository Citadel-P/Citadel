using Application.Permissions;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record DockerVolumeResultView(
    string Id,
    string Name,
    bool InUse,
    string Scope,
    string Driver,
    string Mountpoint,
    string CreatedAt,
    ClusterVolume? ClusterVolume,
    VolumeUsageData? UsageData,
    IEnumerable<ContainerVolumeResult> Containers,
    IReadOnlyDictionary<string, string> Status,
    IReadOnlyDictionary<string, string> Labels,
    IReadOnlyDictionary<string, string> Options,
    BackupCoverageView? BackupCoverage = null,
    VolumeCapabilities? Capabilities = null)
{
    internal static async Task<DockerVolumeResultView> Map(DockerVolumeResult volume, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(volume.PlatformId, ResourceType.Platform);
        return Map(volume) with
        {
            Capabilities = CapabilityMapper.ToVolumeCapabilities(permissions)
        };
    }

    internal static DockerVolumeResultView Map(DockerVolumeResult volume)
        => new(
            Id: volume.Id,
            Name: volume.Name,
            InUse: volume.InUse,
            Scope: volume.Scope,
            Driver: volume.Driver,
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            ClusterVolume: volume.ClusterVolume,
            UsageData: volume.UsageData,
            Containers: volume.Containers,
            Status: volume.Status,
            Labels: volume.Labels,
            Options: volume.Options);
}
