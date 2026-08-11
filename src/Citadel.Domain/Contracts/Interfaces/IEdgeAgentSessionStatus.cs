namespace Domain.Contracts.Interfaces;

public interface IEdgeAgentSessionStatus
{
    bool IsNodeConnected(Guid platformId, string dockerNodeId);
}

public interface ISwarmNodeDataPlaneCoordinator
{
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
