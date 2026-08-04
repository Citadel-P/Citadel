using Domain.Entities.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

public sealed record SwarmServiceView(
    string Id, long VersionIndex, string Name, string Mode, string Image,
    int RunningTaskCount, int DesiredTaskCount, string UpdateState, string? UpdateMessage,
    IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds, IReadOnlyList<string> SecretIds,
    IReadOnlyList<string> ConfigIds, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmServiceView Map(SwarmServiceProjection value) => new(
        value.DockerServiceId, value.VersionIndex, value.Name, value.Mode, value.Image,
        value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
        value.Ports, value.NetworkIds, value.SecretIds, value.ConfigIds, value.Labels,
        value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);
}
public sealed record SwarmServicesView(IReadOnlyList<SwarmServiceView> Items)
{
    public static SwarmServicesView Map(IEnumerable<SwarmServiceProjection> values) => new(values.Select(SwarmServiceView.Map).ToArray());
}

public sealed record SwarmTaskView(
    string Id, long VersionIndex, string Name, string ServiceId, string ServiceName, int? Slot,
    string NodeId, string NodeHostname, string DesiredState, string State, string? StatusMessage,
    string? Error, string Image, IReadOnlyList<string> Ports, DateTimeOffset? StatusTimestamp,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmTaskView Map(SwarmTaskProjection value) => new(
        value.DockerTaskId, value.VersionIndex, value.Name, value.DockerServiceId, value.ServiceName,
        value.Slot, value.DockerNodeId, value.NodeHostname, value.DesiredState, value.State,
        value.StatusMessage, value.Error, value.Image, value.Ports, value.StatusTimestamp,
        value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);
}
public sealed record SwarmTasksView(IReadOnlyList<SwarmTaskView> Items)
{
    public static SwarmTasksView Map(IEnumerable<SwarmTaskProjection> values) => new(values.Select(SwarmTaskView.Map).ToArray());
}

public sealed record SwarmNetworkView(
    string Id, string Name, string Scope, string Driver, bool IsAttachable, bool IsInternal,
    bool IsIngress, bool IsEncrypted, bool EnableIPv6, IReadOnlyList<string> Subnets,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? CreatedAt, DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmNetworkView Map(SwarmNetworkProjection value) => new(
        value.DockerNetworkId, value.Name, value.Scope, value.Driver, value.IsAttachable,
        value.IsInternal, value.IsIngress, value.IsEncrypted, value.EnableIPv6, value.Subnets,
        value.ServiceNames, value.Labels, value.DockerCreatedAt, value.ObservedAt, value.IsStale);
}
public sealed record SwarmNetworksView(IReadOnlyList<SwarmNetworkView> Items)
{
    public static SwarmNetworksView Map(IEnumerable<SwarmNetworkProjection> values) => new(values.Select(SwarmNetworkView.Map).ToArray());
}

public sealed record SwarmSecretView(
    string Id, long VersionIndex, string Name, string? Driver, IReadOnlyList<string> ServiceNames,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmSecretView Map(SwarmSecretProjection value) => new(
        value.DockerSecretId, value.VersionIndex, value.Name, value.Driver, value.ServiceNames,
        value.Labels, value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);
}
public sealed record SwarmSecretsView(IReadOnlyList<SwarmSecretView> Items)
{
    public static SwarmSecretsView Map(IEnumerable<SwarmSecretProjection> values) => new(values.Select(SwarmSecretView.Map).ToArray());
}

public sealed record SwarmConfigView(
    string Id, long VersionIndex, string Name, string? TemplatingDriver, IReadOnlyList<string> ServiceNames,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale)
{
    public static SwarmConfigView Map(SwarmConfigProjection value) => new(
        value.DockerConfigId, value.VersionIndex, value.Name, value.TemplatingDriver, value.ServiceNames,
        value.Labels, value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);
}
public sealed record SwarmConfigsView(IReadOnlyList<SwarmConfigView> Items)
{
    public static SwarmConfigsView Map(IEnumerable<SwarmConfigProjection> values) => new(values.Select(SwarmConfigView.Map).ToArray());
}

public sealed record SwarmInventoryView(
    Guid PlatformId,
    SwarmNodesView Nodes,
    SwarmServicesView Services,
    SwarmTasksView Tasks,
    SwarmNetworksView Networks,
    SwarmSecretsView Secrets,
    SwarmConfigsView Configs)
{
    public static SwarmInventoryView Map(Guid platformId, SwarmProjectionSnapshot value) => new(
        platformId, SwarmNodesView.Map(value.Nodes), SwarmServicesView.Map(value.Services),
        SwarmTasksView.Map(value.Tasks), SwarmNetworksView.Map(value.Networks),
        SwarmSecretsView.Map(value.Secrets), SwarmConfigsView.Map(value.Configs));
}
