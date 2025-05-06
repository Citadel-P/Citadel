using Agent.Server.Volumes;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record InspectVolumeView (
    string Name,
    string Driver,
    string Mountpoint,
    string CreatedAt,
    string Scope,
    bool InUse,
    UsageDataView UsageData,
    ClusterVolumeView ClusterVolume,
    Dictionary<string, string> Labels,
    Dictionary<string, string> Status,
    Dictionary<string, string> Options
    )
{
    internal static InspectVolumeView Map(VolumeReply volume)
    {
        return new InspectVolumeView(
            Name: volume.Name,
            Driver: volume.Driver,
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Scope:volume.Scope,
            InUse: volume.InUse,
            UsageData: volume.UsageData is null ? null : new UsageDataView(volume.UsageData.Size, volume.UsageData.RefCount),
            ClusterVolume: volume.ClusterVolume is null ? null : new ClusterVolumeView(
                Id: volume.ClusterVolume.Id,
                Version: volume.ClusterVolume.Version is null ? null : new VolumVersionView(volume.ClusterVolume.Version.Index),
                CreatedAt: volume.ClusterVolume.CreatedAt,
                UpdatedAt: volume.ClusterVolume.UpdatedAt,
                Spec: volume.ClusterVolume.Spec is null ? null : VolumeSpecView.Map(volume.ClusterVolume.Spec),
                Info: volume.ClusterVolume.Info is null ? null : ClusterVolumeInfoView.Map(volume.ClusterVolume.Info),
                PublishStatus: volume.ClusterVolume.Info is null ? null : volume.ClusterVolume.PublishStatus.Select(s => new PublishStatusView(s.NodeID, s.State, s.PublishContext.ToDictionary(k => k.Key, v => v.Value))).ToList()
            ),
            volume.Labels.ToDictionary(k => k.Key, v => v.Value),
            volume.Status.ToDictionary(k => k.Key, v => v.Value),
            volume.Options.ToDictionary(k => k.Key, v => v.Value)
        );
    }
}

public sealed record ClusterVolumeView(
    string Id,
    VolumVersionView Version,
    string CreatedAt,
    string UpdatedAt,
    VolumeSpecView Spec,
    ClusterVolumeInfoView Info,
    List<PublishStatusView> PublishStatus);

public record UsageDataView(
    long? Size,
    long? RefCount
);

public record VolumVersionView(
    long? Index 
);

public record VolumeSpecView(
    string Group,
    VolumeAccessModeView AccessMode
)
{
    internal static VolumeSpecView Map(VolumeSpecMessage spec)
    {
        return new VolumeSpecView(
            Group: spec.Group,
            AccessMode: spec.AccessMode is null ? null : new VolumeAccessModeView(
                Scope: spec.AccessMode.Scope,
                Sharing: spec.AccessMode.Sharing,
                Secrets: spec.AccessMode.Secrets?.Select(s => new VolumeSecretView(s.Key, s.Secret)).ToList(),
                CapacityRange: spec.AccessMode.CapacityRange is null ? null : new VolumeCapacityRange(
                    spec.AccessMode.CapacityRange.RequiredBytes,
                    spec.AccessMode.CapacityRange.LimitBytes
                ),
                spec.AccessMode.Availability
            )
        );
    }
}

public record VolumeSecretView(
    string Key,
    string Secret
);

public record VolumeCapacityRange(
    long? RequiredBytes,
    long? LimitBytes
);

public record VolumeAccessModeView(
    VolumeScopeType Scope,
    VolumeSharingType Sharing,
    List<VolumeSecretView> Secrets,
    VolumeCapacityRange CapacityRange,
    string Availability
);

public record ClusterVolumeInfoView(
    long? CapacityBytes,
    Dictionary<string, string> VolumeContext,
    string VolumeID,
    List<TopologyEntryView> AccessibleTopology
)
{
    internal static ClusterVolumeInfoView Map(ClusterVolumeInfoMessage info)
    {
        return new ClusterVolumeInfoView(
            VolumeID: info.VolumeID,
            CapacityBytes: info.CapacityBytes,
            VolumeContext: info.VolumeContext?.ToDictionary(k => k.Key, v => v.Value),
            AccessibleTopology: info.AccessibleTopology is null ? null : [.. info.AccessibleTopology.Select(s =>
                new TopologyEntryView(s.Labels.ToDictionary(k => k.Key, v => v.Value))
            )]
        );
    }
}

public record TopologyEntryView(
    Dictionary<string, string> Labels
);

public record PublishStatusView(
    string NodeID,
    string State,
    Dictionary<string, string> PublishContext
);