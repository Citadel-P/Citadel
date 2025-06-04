using System.Text.Json.Serialization;

namespace Infrastructure.Entities;

[method: JsonConstructor]
public class DockerSwarmPlatformDescriptor(
    string nodeID,
    string nodeAddr,
    string localNodeState,
    bool controlAvailable,
    long nodes,
    long managers,
    string daemonId,
    long containerCount,
    long containersRunning,
    long containersPaused,
    long containersStopped,
    string? driver = null,
    string? operatingSystem = null,
    string? osVersion = null,
    string? osType = null,
    string? architecture = null,
    string? error = null,
    IEnumerable<SwarmPeer>? remoteManagers = null
    )  : DockerPlatformDescriptor(
        daemonId: daemonId,
        containerCount: containerCount,
        containersRunning: containersRunning,
        containersPaused: containersPaused,
        containersStopped: containersStopped,
        driver: driver,
        operatingSystem: operatingSystem,
        osVersion: osVersion,
        osType: osType,
        architecture: architecture)
{
    public string? NodeID { get; private set; } = nodeID;
    public string? NodeAddr { get; private set; } = nodeAddr;
    public string? LocalNodeState { get; private set; } = localNodeState;
    public bool ControlAvailable { get; private set; } = controlAvailable;
    public string? Error { get; private set; } = error;
    public long Nodes { get; private set; } = nodes;
    public long Managers { get; private set; } = managers;
    public ICollection<SwarmPeer> RemoteManagers { get; private set; } = remoteManagers?.ToList() ?? [];

    public void PartialUpdate(
        string? nodeID = null,
        string? nodeAddr = null,
        string? localNodeState = null,
        bool? controlAvailable = null,
        string? error = null,
        long? nodes = null,
        long? managers = null)
    {
        if (nodeID != null) NodeID = nodeID;
        if (nodeAddr != null) NodeAddr = nodeAddr;
        if (localNodeState != null) LocalNodeState = localNodeState;
        if (controlAvailable != null) ControlAvailable = controlAvailable.Value;
        if (error != null) Error = error;
        if (nodes != null) Nodes = nodes.Value;
        if (managers != null) Managers = managers.Value;
    }
}

public sealed record SwarmPeer(string? NodeID, string? Addr);
