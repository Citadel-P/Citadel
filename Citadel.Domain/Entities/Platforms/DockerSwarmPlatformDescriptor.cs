using System.Text.Json.Serialization;

namespace Domain.Entities.Platforms;

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
    string? Error = null,
    IEnumerable<SwarmPeer>? RemoteManagers = null
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
        Architecture: Architecture)
{
    public DockerSwarmPlatformDescriptor PartialUpdate(
        string? nodeID = null,
        string? nodeAddr = null,
        string? localNodeState = null,
        bool? controlAvailable = null,
        string? error = null,
        long? nodes = null,
        long? managers = null) => 
        this with
        {
            NodeID = nodeID ?? NodeID,
            NodeAddr = nodeAddr ?? NodeAddr,
            LocalNodeState = localNodeState ?? LocalNodeState,
            ControlAvailable = controlAvailable ?? ControlAvailable,
            Error = error ?? Error,
            Nodes = nodes ?? Nodes,
            Managers = managers ?? Managers
        };
}

public sealed record SwarmPeer(string? NodeID, string? Addr);
