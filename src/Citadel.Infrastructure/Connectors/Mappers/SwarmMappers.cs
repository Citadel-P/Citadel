using Citadel.Swarm.V1;
using DomainSwarmNode = Domain.Contracts.Resources.Swarm.SwarmNodeResult;
using HostingSwarmNode = Hosting.DockerClient.Models.Swarm.SwarmNodeResult;
using DomainSwarmService = Domain.Contracts.Resources.Swarm.SwarmServiceResult;
using DomainSwarmTask = Domain.Contracts.Resources.Swarm.SwarmTaskResult;
using DomainSwarmNetwork = Domain.Contracts.Resources.Swarm.SwarmNetworkResult;
using DomainSwarmSecret = Domain.Contracts.Resources.Swarm.SwarmSecretResult;
using DomainSwarmConfig = Domain.Contracts.Resources.Swarm.SwarmConfigResult;
using HostingSwarmService = Hosting.DockerClient.Models.Swarm.SwarmServiceResult;
using HostingSwarmTask = Hosting.DockerClient.Models.Swarm.SwarmTaskResult;
using HostingSwarmNetwork = Hosting.DockerClient.Models.Swarm.SwarmNetworkResult;
using HostingSwarmSecret = Hosting.DockerClient.Models.Swarm.SwarmSecretResult;
using HostingSwarmConfig = Hosting.DockerClient.Models.Swarm.SwarmConfigResult;
using DomainSwarmLogs = Domain.Contracts.Resources.Swarm.SwarmLogsResult;
using HostingSwarmLogs = Hosting.DockerClient.Models.Swarm.SwarmLogsResult;

namespace Infrastructure.Connectors.Mappers;

internal static class SwarmMappers
{
    public static DomainSwarmNode Map(this HostingSwarmNode source) =>
        new(
            source.Id,
            source.VersionIndex,
            source.Hostname,
            source.Role,
            source.IsLeader,
            source.Reachability,
            source.Status,
            source.StatusMessage,
            source.Availability,
            source.EngineVersion,
            source.OperatingSystem,
            source.Architecture,
            source.Address,
            source.Labels,
            source.RunningTaskCount,
            source.DesiredTaskCount,
            source.CreatedAt,
            source.UpdatedAt);

    public static IReadOnlyList<DomainSwarmNode> Map(this IReadOnlyList<HostingSwarmNode> source)
    {
        var nodes = new DomainSwarmNode[source.Count];
        for (var index = 0; index < source.Count; index++)
            nodes[index] = source[index].Map();
        return nodes;
    }

    public static DomainSwarmNode Map(this SwarmNodeMessage source) =>
        new(
            source.Id,
            checked((long)source.VersionIndex),
            source.Hostname,
            source.Role,
            source.IsLeader,
            source.Reachability,
            source.Status,
            string.IsNullOrWhiteSpace(source.StatusMessage) ? null : source.StatusMessage,
            source.Availability,
            source.EngineVersion,
            source.OperatingSystem,
            source.Architecture,
            source.Address,
            source.Labels,
            source.RunningTaskCount,
            source.DesiredTaskCount,
            source.CreatedAt?.ToDateTimeOffset(),
            source.UpdatedAt?.ToDateTimeOffset());

    public static IReadOnlyList<DomainSwarmNode> Map(this ListSwarmNodesResponse source)
    {
        var nodes = new DomainSwarmNode[source.Nodes.Count];
        for (var index = 0; index < source.Nodes.Count; index++)
            nodes[index] = source.Nodes[index].Map();
        return nodes;
    }

    public static DomainSwarmService Map(this HostingSwarmService source) => new(
        source.Id, source.VersionIndex, source.Name, source.Mode, source.Image,
        source.RunningTaskCount, source.DesiredTaskCount, source.UpdateState, source.UpdateMessage,
        source.Ports, source.NetworkIds, source.SecretIds, source.ConfigIds, source.Labels,
        source.CreatedAt, source.UpdatedAt, source.RuntimeHash, source.ForceUpdate);
    public static IReadOnlyList<DomainSwarmService> Map(this IReadOnlyList<HostingSwarmService> source) => MapList(source, static value => value.Map());
    public static DomainSwarmService Map(this SwarmServiceMessage source) => new(
        source.Id, checked((long)source.VersionIndex), source.Name, source.Mode, source.Image,
        source.RunningTaskCount, source.DesiredTaskCount, source.UpdateState,
        EmptyToNull(source.UpdateMessage), source.Ports.ToArray(), source.NetworkIds.ToArray(),
        source.SecretIds.ToArray(), source.ConfigIds.ToArray(), source.Labels,
        source.CreatedAt?.ToDateTimeOffset(), source.UpdatedAt?.ToDateTimeOffset(), source.RuntimeHash,
        source.ForceUpdate);
    public static IReadOnlyList<DomainSwarmService> Map(this ListSwarmServicesResponse source) => MapList(source.Services, static value => value.Map());

    public static DomainSwarmTask Map(this HostingSwarmTask source) => new(
        source.Id, source.VersionIndex, source.Name, source.ServiceId, source.Slot, source.NodeId,
        source.DesiredState, source.State, source.StatusMessage, source.Error, source.Image,
        source.Ports, source.StatusTimestamp, source.CreatedAt, source.UpdatedAt, source.ContainerId);
    public static IReadOnlyList<DomainSwarmTask> Map(this IReadOnlyList<HostingSwarmTask> source) => MapList(source, static value => value.Map());
    public static DomainSwarmTask Map(this SwarmTaskMessage source) => new(
        source.Id, checked((long)source.VersionIndex), source.Name, source.ServiceId,
        source.HasSlot ? source.Slot : null, source.NodeId, source.DesiredState, source.State,
        EmptyToNull(source.StatusMessage), EmptyToNull(source.Error), source.Image, source.Ports.ToArray(),
        source.StatusTimestamp?.ToDateTimeOffset(), source.CreatedAt?.ToDateTimeOffset(), source.UpdatedAt?.ToDateTimeOffset(),
        EmptyToNull(source.ContainerId));
    public static IReadOnlyList<DomainSwarmTask> Map(this ListSwarmTasksResponse source) => MapList(source.Tasks, static value => value.Map());

    public static DomainSwarmNetwork Map(this HostingSwarmNetwork source) => new(
        source.Id, source.Name, source.Scope, source.Driver, source.IsAttachable, source.IsInternal,
        source.IsIngress, source.IsEncrypted, source.EnableIPv6, source.Subnets, source.Labels, source.CreatedAt);
    public static IReadOnlyList<DomainSwarmNetwork> Map(this IReadOnlyList<HostingSwarmNetwork> source) => MapList(source, static value => value.Map());
    public static DomainSwarmNetwork Map(this SwarmNetworkMessage source) => new(
        source.Id, source.Name, source.Scope, source.Driver, source.IsAttachable, source.IsInternal,
        source.IsIngress, source.IsEncrypted, source.EnableIpv6, source.Subnets.ToArray(), source.Labels,
        source.CreatedAt?.ToDateTimeOffset());
    public static IReadOnlyList<DomainSwarmNetwork> Map(this ListSwarmNetworksResponse source) => MapList(source.Networks, static value => value.Map());

    public static DomainSwarmSecret Map(this HostingSwarmSecret source) => new(
        source.Id, source.VersionIndex, source.Name, source.Driver, source.Labels, source.CreatedAt, source.UpdatedAt);
    public static IReadOnlyList<DomainSwarmSecret> Map(this IReadOnlyList<HostingSwarmSecret> source) => MapList(source, static value => value.Map());
    public static DomainSwarmSecret Map(this SwarmSecretMessage source) => new(
        source.Id, checked((long)source.VersionIndex), source.Name, EmptyToNull(source.Driver), source.Labels,
        source.CreatedAt?.ToDateTimeOffset(), source.UpdatedAt?.ToDateTimeOffset());
    public static IReadOnlyList<DomainSwarmSecret> Map(this ListSwarmSecretsResponse source) => MapList(source.Secrets, static value => value.Map());

    public static DomainSwarmConfig Map(this HostingSwarmConfig source) => new(
        source.Id, source.VersionIndex, source.Name, source.TemplatingDriver, source.Labels, source.CreatedAt, source.UpdatedAt);
    public static IReadOnlyList<DomainSwarmConfig> Map(this IReadOnlyList<HostingSwarmConfig> source) => MapList(source, static value => value.Map());
    public static DomainSwarmConfig Map(this SwarmConfigMessage source) => new(
        source.Id, checked((long)source.VersionIndex), source.Name, EmptyToNull(source.TemplatingDriver), source.Labels,
        source.CreatedAt?.ToDateTimeOffset(), source.UpdatedAt?.ToDateTimeOffset());
    public static IReadOnlyList<DomainSwarmConfig> Map(this ListSwarmConfigsResponse source) => MapList(source.Configs, static value => value.Map());

    public static DomainSwarmLogs Map(this HostingSwarmLogs source) => new(source.Lines, source.Truncated);
    public static DomainSwarmLogs Map(this SwarmLogsResponse source) => new(source.Lines.ToArray(), source.Truncated);

    private static IReadOnlyList<TResult> MapList<TSource, TResult>(IReadOnlyList<TSource> source, Func<TSource, TResult> map)
    {
        var result = new TResult[source.Count];
        for (var index = 0; index < source.Count; index++)
            result[index] = map(source[index]);
        return result;
    }

    private static string? EmptyToNull(string? value) => string.IsNullOrWhiteSpace(value) ? null : value;
}
