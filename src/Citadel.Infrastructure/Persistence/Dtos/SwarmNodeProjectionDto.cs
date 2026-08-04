namespace Infrastructure.Persistence.Dtos;

internal sealed record SwarmNodeProjectionDto(
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
    string Labels,
    int RunningTaskCount,
    int DesiredTaskCount,
    DateTime? DockerCreatedAt,
    DateTime? DockerUpdatedAt,
    DateTime ObservedAt,
    bool IsStale);
