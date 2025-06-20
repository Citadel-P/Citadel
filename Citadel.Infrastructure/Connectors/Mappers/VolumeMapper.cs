using Citadel.Agent.Volumes.V1;
using Domain;
using Domain.Contracts.Resources.Volumes;

namespace Infrastructure.Connectors.Mappers;

internal static class VolumeMapper
{
    public static IEnumerable<DockerVolumeResult> Map(this ListVolumesResponse response)
    {
        return response.Volumes.Select(Map);
    }

    public static DockerVolumeResult Map(this VolumeResponse volume)
    {
        return new DockerVolumeResult(
            Id: volume.Name,
            Driver: volume.Driver,
            Labels: volume.Labels.ToDictionary(kv => kv.Key, kv => kv.Value),
            Options: volume.Options.ToDictionary(kv => kv.Key, kv => kv.Value),
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Status: volume.Status.ToDictionary(kv => kv.Key, kv => kv.Value),
            Scope: volume.Scope,
            ClusterVolume: volume.ClusterVolume?.Map(),
            UsageData: volume.UsageData?.Map(),
            InUse: volume.InUse);
    }

    public static VolumeUsageData? Map(this UsageDataMessage usageData) 
        => new 
        (
            Size: usageData.Size,
            RefCount: usageData.RefCount
        );

    public static ClusterVolume? Map(this ClusterVolumeMessage? clusterVolume)
    {
        return clusterVolume == null ? null : new ClusterVolume(
            Id: clusterVolume.Id,
            Version: clusterVolume.Version?.Map(),
            CreatedAt: clusterVolume.CreatedAt,
            UpdatedAt: clusterVolume.UpdatedAt,
            Spec: clusterVolume.Spec?.Map(),
            Info: clusterVolume.Info?.Map(),
            PublishStatus: clusterVolume.PublishStatus?.Select(Map).ToList() ?? []);
    }

    public static VolumePublishStatus Map(this PublishStatusMessage status)
    {
        return new VolumePublishStatus
        (
            NodeID: status.NodeID,
            State: status.State,
            PublishContext: status.PublishContext
        );
    }

    public static ClusterVolumeInfo? Map(this ClusterVolumeInfoMessage info)
    {
        return new ClusterVolumeInfo
        (
            VolumeID: info.VolumeID,
            CapacityBytes: info.CapacityBytes,
            VolumeContext: info.VolumeContext.ToDictionary(kv => kv.Key, kv => kv.Value),
            AccessibleTopology: [.. info.AccessibleTopology.Select(t => new TopologyEntry(t.Labels.ToDictionary(kv => kv.Key, kv => kv.Value)))]
        );
    }

    public static VolumeVersionInfo? Map(this VolumVersionMessage versionInfo)
    => new (Index: versionInfo.Index);

   public static VolumeSpecification? Map(this VolumeSpecMessage spec)
    {
        return new VolumeSpecification(Group: spec.Group, AccessMode: spec.AccessMode?.Map());
    }
    public static VolumeAccessMode Map(this VolumeAccessModeMessage accessMode)
        => new 
        (
            Scope: accessMode.Scope.Map(),
            Sharing: accessMode.Sharing.Map(),
            Availability: accessMode.Availability,
            CapacityRange: accessMode.CapacityRange?.Map(),
            Secrets: accessMode.Secrets?.Select(Map)?.ToList() ?? []
            );

    public static Domain.Contracts.Resources.Volumes.VolumeCapacityRange? Map(this Citadel.Agent.Volumes.V1.VolumeCapacityRange capacityRange)
        => new
        (
            RequiredBytes: capacityRange.RequiredBytes,
            LimitBytes: capacityRange.LimitBytes
        );

    public static VolumeSecret Map(this VolumeSecretMessage secret)
        => new
        (
            Key: secret.Key,
            Secret: secret.Secret
        );

    public static VolumeScope Map(this VolumeScopeType scope)
        => scope switch
        {
            VolumeScopeType.Multi => VolumeScope.Multi,
            VolumeScopeType.Single => VolumeScope.Single,
            _ => throw new ArgumentOutOfRangeException(nameof(scope), scope, null)
        };

    public static VolumeSharing Map(this VolumeSharingType type)
        => type switch
        {
            VolumeSharingType.All => VolumeSharing.All,
            VolumeSharingType.Readonly => VolumeSharing.ReadOnly,
            VolumeSharingType.Onewriter => VolumeSharing.OneWriter,
            VolumeSharingType.None => VolumeSharing.None,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };
}
