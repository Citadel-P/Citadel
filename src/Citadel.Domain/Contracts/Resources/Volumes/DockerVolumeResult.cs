using Domain.Entities;

namespace Domain.Contracts.Resources.Volumes;

public record DockerVolumeResult (
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
    IReadOnlyDictionary<string, string> Options)
{
    public Guid PlatformId { get; set; }
    public string? DockerNodeId { get; set; }
    public string? NodeHostname { get; set; }
    public bool IsStale { get; set; }
    public string? StaleReason { get; set; }
}

public record ContainerVolumeResult(
    string Id,
    string Name,
    string Image,
    string ImageId,
    ContainerStateStatus State,
    Dictionary<string, string> Networks,
    Dictionary<string, IReadOnlyList<HostPortBinding>> Ports
    );

public record TopologyEntry(IReadOnlyDictionary<string, string> Labels);

public record ClusterVolumeInfo(
    long? CapacityBytes,
    IReadOnlyDictionary<string, string> VolumeContext,
    string VolumeID,
    IReadOnlyList<TopologyEntry> AccessibleTopology);
