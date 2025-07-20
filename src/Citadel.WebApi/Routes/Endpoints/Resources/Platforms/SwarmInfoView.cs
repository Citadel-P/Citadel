namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record SwarmInfoView(
     Guid Id,
     string? NodeID,
     string? NodeAddr,
     string? LocalNodeState,
     bool ControlAvailable,
     string? Error,
     long Nodes,
     long Managers,
     IList<SwarmPeerView>? RemoteManagers
    );

public record struct SwarmPeerView(
    string? NodeID,
    string? Addr);