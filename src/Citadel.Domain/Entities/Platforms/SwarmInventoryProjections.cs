using Domain.Contracts.Resources.Swarm;

namespace Domain.Entities.Platforms;

public sealed record SwarmServiceProjection(
    Guid PlatformId, string DockerServiceId, long VersionIndex, string Name, string Mode,
    string Image, int RunningTaskCount, int DesiredTaskCount, string UpdateState,
    string? UpdateMessage, IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds,
    IReadOnlyList<string> SecretIds, IReadOnlyList<string> ConfigIds,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmServiceProjection FromObservation(Guid platformId, SwarmServiceResult value, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.Mode, value.Image,
            value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
            value.Ports, value.NetworkIds, value.SecretIds, value.ConfigIds, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false);
}

public sealed record SwarmTaskProjection(
    Guid PlatformId, string DockerTaskId, long VersionIndex, string Name, string DockerServiceId,
    string ServiceName, int? Slot, string DockerNodeId, string NodeHostname, string DesiredState,
    string State, string? StatusMessage, string? Error, string Image, IReadOnlyList<string> Ports,
    DateTimeOffset? StatusTimestamp, DateTimeOffset? DockerCreatedAt, DateTimeOffset? DockerUpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmTaskProjection FromObservation(
        Guid platformId, SwarmTaskResult value, string serviceName, string nodeHostname, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.ServiceId, serviceName,
            value.Slot, value.NodeId, nodeHostname, value.DesiredState, value.State, value.StatusMessage,
            value.Error, value.Image, value.Ports, value.StatusTimestamp, value.CreatedAt, value.UpdatedAt,
            observedAt, false);
}

public sealed record SwarmNetworkProjection(
    Guid PlatformId, string DockerNetworkId, string Name, string Scope, string Driver,
    bool IsAttachable, bool IsInternal, bool IsIngress, bool IsEncrypted, bool EnableIPv6,
    IReadOnlyList<string> Subnets, IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? DockerCreatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmNetworkProjection FromObservation(Guid platformId, SwarmNetworkResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.Name, value.Scope, value.Driver, value.IsAttachable,
            value.IsInternal, value.IsIngress, value.IsEncrypted, value.EnableIPv6, value.Subnets, serviceNames,
            value.Labels, value.CreatedAt, observedAt, false);
}

public sealed record SwarmSecretProjection(
    Guid PlatformId, string DockerSecretId, long VersionIndex, string Name, string? Driver,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmSecretProjection FromObservation(Guid platformId, SwarmSecretResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.Driver, serviceNames, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false);
}

public sealed record SwarmConfigProjection(
    Guid PlatformId, string DockerConfigId, long VersionIndex, string Name, string? TemplatingDriver,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels, DateTimeOffset? DockerCreatedAt,
    DateTimeOffset? DockerUpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmConfigProjection FromObservation(Guid platformId, SwarmConfigResult value, IReadOnlyList<string> serviceNames, DateTimeOffset observedAt) =>
        new(platformId, value.Id, value.VersionIndex, value.Name, value.TemplatingDriver, serviceNames, value.Labels,
            value.CreatedAt, value.UpdatedAt, observedAt, false);
}

public sealed record SwarmProjectionSnapshot(
    IReadOnlyList<SwarmNodeProjection> Nodes,
    IReadOnlyList<SwarmServiceProjection> Services,
    IReadOnlyList<SwarmTaskProjection> Tasks,
    IReadOnlyList<SwarmNetworkProjection> Networks,
    IReadOnlyList<SwarmSecretProjection> Secrets,
    IReadOnlyList<SwarmConfigProjection> Configs);
