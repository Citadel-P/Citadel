namespace Infrastructure.Entities;

public class SwarmInfo
{
    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string? NodeID { get; private set; }
    public string? NodeAddr { get; private set; }
    public string? LocalNodeState { get; private set; }
    public bool ControlAvailable { get; private set; }
    public string? Error { get; private set; }
    public long Nodes { get; private set; }
    public long Managers { get; private set; }
    public ICollection<SwarmPeer> RemoteManagers { get; private set; } = [];

    /// <summary>
    /// EF navigation
    /// </summary>
    public Platform Platform { get; private set; }

    public static SwarmInfo Create(
        string? nodeID,
        string? nodeAddr,
        string? localNodeState,
        bool controlAvailable,
        string? error,
        long nodes,
        long managers,
        IEnumerable<SwarmPeer>? remoteManagers)
        => new()
        {
            Id = Guid.CreateVersion7(),
            NodeID = nodeID,
            NodeAddr = nodeAddr,
            LocalNodeState = localNodeState,
            ControlAvailable = controlAvailable,
            Error = error,
            Nodes = nodes,
            Managers = managers,
            RemoteManagers = remoteManagers is not null ? remoteManagers.ToList() : [],
        };
}