using Citadel.SharedModels.V1;
using Citadel.Volumes.V1;
using Domain;
using Domain.Contracts.Resources.Volumes;
using Google.Protobuf.Collections;
using Hosting.DockerClient.Models.Volumes;

namespace Infrastructure.Connectors.Mappers;

internal static class VolumeMappers
{
    internal static IEnumerable<DockerVolumeResult> Map(this ListVolumesResponse volumesResponse) 
        => volumesResponse.Volumes.Select(Map);

    internal static IEnumerable<DockerVolumeResult> Map(this ListVolumesResult volumesResult)
        => volumesResult.Volumes.Select(Map);

    internal static DockerVolumeResult Map(this VolumeResponse volume)
    {
        return new DockerVolumeResult(
            Id: volume.Name,
            Name: volume.Name,
            Driver: volume.Driver,
            Labels: volume.Labels.ToDictionary(kv => kv.Key, kv => kv.Value),
            Options: volume.Options.ToDictionary(kv => kv.Key, kv => kv.Value),
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Status: volume.Status.ToDictionary(kv => kv.Key, kv => kv.Value),
            Scope: volume.Scope,
            ClusterVolume: volume.ClusterVolume?.Map(),
            UsageData: volume.UsageData?.Map(),
            Containers: volume.Containers?.Map() ?? [],
            InUse: volume.InUse);
    }
    internal static IEnumerable<Domain.Contracts.Resources.Volumes.ContainerVolumeResult> Map(this RepeatedField<Citadel.SharedModels.V1.ContainerVolumeResult>? containers)
        => containers?.Select(Map) ?? [];

    internal static Domain.Contracts.Resources.Volumes.ContainerVolumeResult Map(this Citadel.SharedModels.V1.ContainerVolumeResult container)
        => new
        (
            Id: container.Id,
            Name: container.Name,
            Image: container.Image,
            ImageId: container.ImageId,
            State: container.State.Map(),
            Networks: container.Networks?.ToDictionary() ?? [],
            Ports: container.Ports?.Map() ?? []
        );

    internal static DockerVolumeResult Map(this VolumeResult volume)
    {
        return new DockerVolumeResult
        (
            Id: volume.Name,
            Name: volume.Name,
            InUse: volume.InUse,
            Scope: volume.Scope,
            Driver: volume.Driver,
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Containers: volume.Containers?.Map() ?? [],
            ClusterVolume: volume.ClusterVolume?.Map(),
            UsageData: volume.UsageData?.Map(),
            Status: volume.Status.ToDictionary(kv => kv.Key, kv => kv.Value),
            Labels: volume.Labels.ToDictionary(kv => kv.Key, kv => kv.Value),
            Options: volume.Options.ToDictionary(kv => kv.Key, kv => kv.Value)
        );
    }

    internal static IEnumerable<Domain.Contracts.Resources.Volumes.ContainerVolumeResult> Map(this IEnumerable<ContainerVolume>? containers)
        => containers?.Select(Map) ?? [];

    internal static Domain.Contracts.Resources.Volumes.ContainerVolumeResult Map(this ContainerVolume container)
        => new
        (
            Id: container.Id,
            Name: container.Name,
            Image: container.Image,
            ImageId: container.ImageId,
            State: container.State.Map(),
            Networks: container.Networks ?? [],
            Ports: container.Ports?.Map() ?? []
        );

    internal static VolumeUsageData? Map(this UsageDataMessage usageData) 
        => new 
        (
            Size: usageData.Size,
            RefCount: usageData.RefCount
        );

    private static VolumeUsageData? Map(this Hosting.DockerClient.UsageData usageData)
        => new
        (
            Size: usageData.Size,
            RefCount: usageData.RefCount
        );

    internal static ClusterVolume? Map(this ClusterVolumeMessage? clusterVolume)
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

    private static ClusterVolume Map(this Hosting.DockerClient.ClusterVolume clusterVolume)
    {
        return new ClusterVolume(
            Id: clusterVolume.ID,
            Version: clusterVolume.Version?.Map(),
            CreatedAt: clusterVolume.CreatedAt,
            UpdatedAt: clusterVolume.UpdatedAt,
            Spec: clusterVolume.Spec?.Map(),
            Info: clusterVolume.Info?.Map(),
            PublishStatus: clusterVolume.PublishStatus?.Select(Map).ToList() ?? []);
    }

    internal static VolumePublishStatus Map(this PublishStatusMessage status)
    {
        return new VolumePublishStatus
        (
            NodeID: status.NodeID,
            State: status.State,
            PublishContext: status.PublishContext
        );
    }

    private static VolumePublishStatus Map(this Hosting.DockerClient.PublishStatus status)
    {
        return new VolumePublishStatus
        (
            NodeID: status.NodeID,
            State: status.State?.ToString() ?? "unknown",
            PublishContext: status.PublishContext?.ToDictionary() ?? []
        );
    }

    internal static ClusterVolumeInfo? Map(this ClusterVolumeInfoMessage info)
    {
        return new ClusterVolumeInfo
        (
            VolumeID: info.VolumeID,
            CapacityBytes: info.CapacityBytes,
            VolumeContext: info.VolumeContext.ToDictionary(kv => kv.Key, kv => kv.Value),
            AccessibleTopology: [.. info.AccessibleTopology.Select(t => new TopologyEntry(t.Labels.ToDictionary(kv => kv.Key, kv => kv.Value)))]
        );
    }

    private static ClusterVolumeInfo? Map(this Hosting.DockerClient.Info info)
    {
        return new ClusterVolumeInfo
        (
            VolumeID: info.VolumeID,
            CapacityBytes: info.CapacityBytes,
            VolumeContext: info.VolumeContext.ToDictionary(kv => kv.Key, kv => kv.Value),
            AccessibleTopology: [.. info.AccessibleTopology.Select(t => new TopologyEntry(t.ToDictionary(kv => kv.Key, kv => kv.Value)))]
        );
    }

    internal static VolumeVersionInfo? Map(this VolumVersionMessage versionInfo)
        => new (Index: versionInfo.Index);

    internal static VolumeVersionInfo? Map(this Hosting.DockerClient.ObjectVersion versionInfo)
       => new(Index: versionInfo.Index != null ? (long)versionInfo.Index.Value : 0);

    internal static VolumeSpecification? Map(this VolumeSpecMessage spec) 
        => new (Group: spec.Group, AccessMode: spec.AccessMode?.Map());

    private static VolumeSpecification? Map(this Hosting.DockerClient.ClusterVolumeSpec spec)
       => new(Group: spec.Group, AccessMode: spec.AccessMode?.Map());

    private static VolumeAccessMode Map(this VolumeAccessModeMessage accessMode)
        => new
        (
            Scope: accessMode.Scope.Map(),
            Sharing: accessMode.Sharing.Map(),
            Availability: accessMode.Availability,
            CapacityRange: accessMode.CapacityRange?.Map(),
            Secrets: accessMode.Secrets?.Select(Map)?.ToList() ?? []
        );

    private static VolumeAccessMode Map(this Hosting.DockerClient.AccessMode accessMode)
        => new 
        (
            Scope: accessMode.Scope.Map(),
            Sharing: accessMode.Sharing.Map(),
            Availability: accessMode.Availability?.ToString(),
            CapacityRange: accessMode.CapacityRange?.Map(),
            Secrets: accessMode.Secrets?.Select(Map)?.ToList() ?? []
        );

    internal static Domain.Contracts.Resources.Volumes.VolumeCapacityRange? Map(this Citadel.SharedModels.V1.VolumeCapacityRange capacityRange)
        => new
        (
            RequiredBytes: capacityRange.RequiredBytes,
            LimitBytes: capacityRange.LimitBytes
        );

    private static Domain.Contracts.Resources.Volumes.VolumeCapacityRange? Map(this Hosting.DockerClient.CapacityRange capacityRange)
       => new
       (
           RequiredBytes: capacityRange.RequiredBytes,
           LimitBytes: capacityRange.LimitBytes
       );

    private static VolumeSecret Map(this Hosting.DockerClient.Secrets2 secret)
        => new
        (
            Key: secret.Key,
            Secret: secret.Secret
        );

    internal static VolumeSecret Map(this VolumeSecretMessage secret)
        => new
        (
            Key: secret.Key,
            Secret: secret.Secret
        );

    private  static VolumeScope Map(this Hosting.DockerClient.AccessModeScope? scope)
       => scope switch
       {
           Hosting.DockerClient.AccessModeScope.Multi => VolumeScope.Multi,
           Hosting.DockerClient.AccessModeScope.Single => VolumeScope.Single,
           _ => throw new ArgumentOutOfRangeException(nameof(scope), scope, null)
       };

    private static VolumeSharing Map(this Hosting.DockerClient.AccessModeSharing? type)
       => type switch
       {
           Hosting.DockerClient.AccessModeSharing.All => VolumeSharing.All,
           Hosting.DockerClient.AccessModeSharing.Readonly => VolumeSharing.ReadOnly,
           Hosting.DockerClient.AccessModeSharing.Onewriter => VolumeSharing.OneWriter,
           Hosting.DockerClient.AccessModeSharing.None => VolumeSharing.None,
           _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
       };

    internal static VolumeScope Map(this VolumeScopeType scope)
        => scope switch
        {
            VolumeScopeType.Multi => VolumeScope.Multi,
            VolumeScopeType.Single => VolumeScope.Single,
            _ => throw new ArgumentOutOfRangeException(nameof(scope), scope, null)
        };

    internal static VolumeSharing Map(this VolumeSharingType type)
        => type switch
        {
            VolumeSharingType.All => VolumeSharing.All,
            VolumeSharingType.Readonly => VolumeSharing.ReadOnly,
            VolumeSharingType.Onewriter => VolumeSharing.OneWriter,
            VolumeSharingType.None => VolumeSharing.None,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };

}
