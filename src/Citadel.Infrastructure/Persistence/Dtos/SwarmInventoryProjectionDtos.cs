namespace Infrastructure.Persistence.Dtos;

internal sealed record SwarmServiceProjectionDto(
    Guid PlatformId, string DockerServiceId, long VersionIndex, string Name, string Mode,
    string Image, int RunningTaskCount, int DesiredTaskCount, string UpdateState,
    string? UpdateMessage, string Ports, string NetworkIds, string SecretIds, string ConfigIds,
    string Labels, DateTime? DockerCreatedAt, DateTime? DockerUpdatedAt,
    DateTime ObservedAt, bool IsStale);

internal sealed record SwarmTaskProjectionDto(
    Guid PlatformId, string DockerTaskId, long VersionIndex, string Name, string DockerServiceId,
    string ServiceName, int? Slot, string DockerNodeId, string NodeHostname, string DesiredState,
    string State, string? StatusMessage, string? Error, string Image, string Ports,
    DateTime? StatusTimestamp, DateTime? DockerCreatedAt, DateTime? DockerUpdatedAt,
    DateTime ObservedAt, bool IsStale);

internal sealed record SwarmNetworkProjectionDto(
    Guid PlatformId, string DockerNetworkId, string Name, string Scope, string Driver,
    bool IsAttachable, bool IsInternal, bool IsIngress, bool IsEncrypted, bool EnableIPv6,
    string Subnets, string ServiceNames, string Labels, DateTime? DockerCreatedAt, DateTime ObservedAt, bool IsStale);

internal sealed record SwarmSecretProjectionDto(
    Guid PlatformId, string DockerSecretId, long VersionIndex, string Name, string? Driver,
    string ServiceNames, string Labels, DateTime? DockerCreatedAt, DateTime? DockerUpdatedAt,
    DateTime ObservedAt, bool IsStale);

internal sealed record SwarmConfigProjectionDto(
    Guid PlatformId, string DockerConfigId, long VersionIndex, string Name, string? TemplatingDriver,
    string ServiceNames, string Labels, DateTime? DockerCreatedAt, DateTime? DockerUpdatedAt,
    DateTime ObservedAt, bool IsStale);
