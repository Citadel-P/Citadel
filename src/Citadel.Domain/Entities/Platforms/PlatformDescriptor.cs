using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Platforms;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DockerPlatformDescriptor), nameof(PlatformType.Docker))]
[JsonDerivedType(typeof(DockerSwarmPlatformDescriptor), nameof(PlatformType.DockerSwarm))]
[JsonDerivedType(typeof(KubernetesPlatformDescriptor), nameof(PlatformType.Kubernetes))]
public abstract record PlatformDescriptor
{
    [JsonIgnore]
    public PlatformType Type => this switch
    {
        DockerSwarmPlatformDescriptor => PlatformType.DockerSwarm,
        DockerPlatformDescriptor => PlatformType.Docker,
        KubernetesPlatformDescriptor => PlatformType.Kubernetes,
        _ => throw new InvalidOperationException($"Unsupported platform descriptor '{GetType().Name}'.")
    };
}
[method: JsonConstructor]
public record DockerPlatformDescriptor(
        string DaemonId,
        long ContainerCount,
        long ContainersRunning,
        long ContainersPaused,
        long ContainersStopped,
        string? Driver = null,
        string? OperatingSystem = null,
        string? OsVersion = null,
        string? OsType = null,
        string? Architecture = null,
        long? ImageUsedBytes = null,
        long? VolumeUsedBytes = null,
        string? ApiVersion = null,
        string? MinimumApiVersion = null) : PlatformDescriptor
{
    public DockerPlatformDescriptor Create(
        string? daemonId = null,
        long? containerCount = null,
        long? containersRunning = null,
        long? containersPaused = null,
        long? containersStopped = null,
        string? driver = null,
        string? operatingSystem = null,
        string? osVersion = null,
        string? osType = null,
        string? architecture = null)
        =>
        this with
        {
            DaemonId = daemonId ?? DaemonId,
            ContainerCount = containerCount ?? ContainerCount,
            ContainersRunning = containersRunning ?? ContainersRunning,
            ContainersPaused = containersPaused ?? ContainersPaused,
            ContainersStopped = containersStopped ?? ContainersStopped,
            Driver = driver ?? Driver,
            OperatingSystem = operatingSystem ?? OperatingSystem,
            OsVersion = osVersion ?? OsVersion,
            OsType = osType ?? OsType,
            Architecture = architecture ?? Architecture
        };
}

[method: JsonConstructor]
public sealed record DockerSwarmPlatformDescriptor(
    string NodeID,
    string NodeAddr,
    string LocalNodeState,
    bool ControlAvailable,
    long Nodes,
    long Managers,
    string DaemonId,
    long ContainerCount,
    long ContainersRunning,
    long ContainersPaused,
    long ContainersStopped,
    string? Driver = null,
    string? OperatingSystem = null,
    string? OsVersion = null,
    string? OsType = null,
    string? Architecture = null,
    long? ImageUsedBytes = null,
    long? VolumeUsedBytes = null,
    string? ApiVersion = null,
    string? MinimumApiVersion = null,
    string? ClusterId = null,
    DateTimeOffset? ClusterCreatedAt = null,
    string? Error = null,
    IEnumerable<SwarmPeer>? RemoteManagers = null,
    long? ServiceCount = null,
    long? RunningTaskCount = null
    ) : DockerPlatformDescriptor(
        DaemonId: DaemonId,
        ContainerCount: ContainerCount,
        ContainersRunning: ContainersRunning,
        ContainersPaused: ContainersPaused,
        ContainersStopped: ContainersStopped,
        Driver: Driver,
        OperatingSystem: OperatingSystem,
        OsVersion: OsVersion,
        OsType: OsType,
        Architecture: Architecture,
        ImageUsedBytes: ImageUsedBytes,
        VolumeUsedBytes: VolumeUsedBytes,
        ApiVersion: ApiVersion,
        MinimumApiVersion: MinimumApiVersion)
{
    public DockerSwarmPlatformDescriptor PartialUpdate(
        string? nodeID = null,
        string? nodeAddr = null,
        string? localNodeState = null,
        bool? controlAvailable = null,
        string? error = null,
        long? nodes = null,
        long? managers = null,
        long? serviceCount = null,
        long? runningTaskCount = null) =>
        this with
        {
            NodeID = nodeID ?? NodeID,
            NodeAddr = nodeAddr ?? NodeAddr,
            LocalNodeState = localNodeState ?? LocalNodeState,
            ControlAvailable = controlAvailable ?? ControlAvailable,
            Error = error ?? Error,
            Nodes = nodes ?? Nodes,
            Managers = managers ?? Managers,
            ServiceCount = serviceCount ?? ServiceCount,
            RunningTaskCount = runningTaskCount ?? RunningTaskCount
        };
}

public sealed record SwarmPeer(string? NodeID, string? Addr);

public record KubernetesPlatformDescriptor(
    string? ClusterName,
    string? ClusterVersion,
    string? ApiServerUrl,
    string? Namespace
    ) : PlatformDescriptor;
