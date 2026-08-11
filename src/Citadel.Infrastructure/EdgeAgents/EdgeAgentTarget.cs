using Domain;

namespace Infrastructure.EdgeAgents;

internal readonly record struct EdgeAgentTarget(
    EdgeAgentResourceType ResourceType,
    Guid ResourceId,
    string? DockerNodeId = null)
{
    public static EdgeAgentTarget Platform(Guid platformId) => new(EdgeAgentResourceType.Platform, platformId);

    public static EdgeAgentTarget SwarmNode(Guid platformId, string dockerNodeId)
        => new(EdgeAgentResourceType.Platform, platformId, dockerNodeId);
}
