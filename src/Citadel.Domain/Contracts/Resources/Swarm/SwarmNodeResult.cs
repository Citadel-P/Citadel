namespace Domain.Contracts.Resources.Swarm;

public sealed record SwarmNodeResult(
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
    DateTimeOffset? UpdatedAt);
