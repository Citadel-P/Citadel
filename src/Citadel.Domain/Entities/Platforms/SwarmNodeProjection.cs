using Domain.Contracts.Resources.Swarm;

namespace Domain.Entities.Platforms;

public sealed record SwarmNodeProjection(
    Guid PlatformId,
    string DockerNodeId,
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
    DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt,
    DateTimeOffset ObservedAt,
    bool IsStale)
{
    public static SwarmNodeProjection FromObservation(
        Guid platformId,
        SwarmNodeResult node,
        DateTimeOffset observedAt) =>
        new(
            platformId,
            node.Id,
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
            node.CreatedAt,
            node.UpdatedAt,
            observedAt,
            IsStale: false);
}
