namespace WebApi.Controllers.V1.Resources.Platforms;

public sealed record SwarmInfoView(
    Guid Id,
    string NodeID,
    string NodeAddr,
    string LocalNodeState,
    bool ControlAvailable,
    string Error,
    long Nodes,
    long Managers,
    IList<SwarmPeerView> RemoteManagers
    );

public sealed record SwarmPeerView(string NodeID, string Addr);