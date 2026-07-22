using Domain;

namespace Infrastructure.EdgeAgents;

internal readonly record struct EdgeAgentTarget(EdgeAgentResourceType ResourceType, Guid ResourceId)
{
    public static EdgeAgentTarget Platform(Guid platformId) => new(EdgeAgentResourceType.Platform, platformId);
}
