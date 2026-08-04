namespace Domain.Contracts.Resources.Swarm;

public sealed record ListSwarmNodesCommand(
    string PlatformAddress,
    int Limit = SwarmInventoryLimits.MaximumItems,
    bool IncludeTaskCounts = true);
