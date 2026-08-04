using Domain.Entities.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

public sealed record SwarmNodeView(
    string Id,
    long VersionIndex,
    string Hostname,
    string Role,
    bool IsLeader,
    string Reachability,
    string Status,
    string? StatusMessage,
    string Availability,
    string EngineVersion,
    string OperatingSystem,
    string Architecture,
    string Address,
    IReadOnlyDictionary<string, string> Labels,
    int RunningTaskCount,
    int DesiredTaskCount,
    DateTimeOffset? CreatedAt,
    DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt,
    bool IsStale)
{
    public static SwarmNodeView Map(SwarmNodeProjection node) =>
        new(
            node.DockerNodeId,
            node.VersionIndex,
            node.Hostname,
            node.Role,
            node.IsLeader,
            node.Reachability,
            node.Status,
            node.StatusMessage,
            node.Availability,
            node.EngineVersion,
            node.OperatingSystem,
            node.Architecture,
            node.Address,
            node.Labels,
            node.RunningTaskCount,
            node.DesiredTaskCount,
            node.DockerCreatedAt,
            node.DockerUpdatedAt,
            node.ObservedAt,
            node.IsStale);
}

public sealed record SwarmNodesView(IReadOnlyList<SwarmNodeView> Items)
{
    public static SwarmNodesView Map(IEnumerable<SwarmNodeProjection> nodes) =>
        new(nodes.Select(SwarmNodeView.Map).ToArray());
}
