namespace Domain.Contracts.Resources.Swarm;

public sealed record InspectSwarmNodeCommand(string PlatformAddress, string NodeId);
