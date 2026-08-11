namespace Domain.Contracts.Interfaces;

public interface IEdgeAgentSessionStatus
{
    bool IsNodeConnected(Guid platformId, string dockerNodeId);
}

public interface ISwarmNodeDataPlaneCoordinator
{
    bool HandlesManagerLocalResources(Guid platformId);
    ValueTask NotifyManagerConnectedAsync(Guid platformId, CancellationToken cancellationToken);
    ValueTask NotifyManagerDisconnectedAsync(Guid platformId, CancellationToken cancellationToken);
    ValueTask NotifyManagerDaemonEventAsync(
        Guid platformId,
        Domain.Contracts.Resources.Containers.DaemonEventInfo daemonEvent,
        CancellationToken cancellationToken);

    ValueTask NotifyConnectedAsync(
        Guid platformId,
        string dockerNodeId,
        string sessionId,
        CancellationToken cancellationToken);

    ValueTask NotifyDisconnectedAsync(
        Guid platformId,
        string dockerNodeId,
        string sessionId,
        CancellationToken cancellationToken);
}
