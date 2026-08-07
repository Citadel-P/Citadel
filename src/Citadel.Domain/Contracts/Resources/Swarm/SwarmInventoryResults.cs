using Domain.Entities;
using Domain.Entities.SwarmServices;

namespace Domain.Contracts.Resources.Swarm;

public sealed record SwarmServiceResult(
    string Id, long VersionIndex, string Name, string Mode, string Image,
    int RunningTaskCount, int DesiredTaskCount, string UpdateState, string? UpdateMessage,
    IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds,
    IReadOnlyList<string> SecretIds, IReadOnlyList<string> ConfigIds,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt,
    string RuntimeHash = "", long ForceUpdate = 0, SwarmServiceSpec? Definition = null,
    IReadOnlyList<string>? AdoptionWarnings = null);

public sealed record SwarmTaskResult(
    string Id, long VersionIndex, string Name, string ServiceId, int? Slot, string NodeId,
    string DesiredState, string State, string? StatusMessage, string? Error, string Image,
    IReadOnlyList<string> Ports, DateTimeOffset? StatusTimestamp,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt, string? ContainerId = null);

public sealed record SwarmTaskStatsResult(
    string DockerContainerId,
    IReadOnlyList<ContainerStat> Stats);

public sealed record SwarmNetworkResult(
    string Id, string Name, string Scope, string Driver, bool IsAttachable, bool IsInternal,
    bool IsIngress, bool IsEncrypted, bool EnableIPv6, IReadOnlyList<string> Subnets,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt);

public sealed record SwarmSecretResult(
    string Id, long VersionIndex, string Name, string? Driver,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt);

public sealed record SwarmConfigResult(
    string Id, long VersionIndex, string Name, string? TemplatingDriver,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt);

public sealed record SwarmLogsResult(IReadOnlyList<string> Lines, bool Truncated);
