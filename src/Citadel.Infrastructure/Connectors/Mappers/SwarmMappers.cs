using Citadel.Swarm.V1;
using Domain;
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
using HostingSwarmServiceSpec = Hosting.DockerClient.Models.Swarm.SwarmServiceMutationSpec;
using Domain.Entities.SwarmServices;

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
        source.CreatedAt, source.UpdatedAt, source.RuntimeHash, source.ForceUpdate,
        MapDefinition(source.Definition), source.AdoptionWarnings ?? []);
    public static IReadOnlyList<DomainSwarmService> Map(this IReadOnlyList<HostingSwarmService> source) => MapList(source, static value => value.Map());
    public static DomainSwarmService Map(this SwarmServiceMessage source) => new(
        source.Id, checked((long)source.VersionIndex), source.Name, source.Mode, source.Image,
        source.RunningTaskCount, source.DesiredTaskCount, source.UpdateState,
        EmptyToNull(source.UpdateMessage), source.Ports.ToArray(), source.NetworkIds.ToArray(),
        source.SecretIds.ToArray(), source.ConfigIds.ToArray(), source.Labels,
        source.CreatedAt?.ToDateTimeOffset(), source.UpdatedAt?.ToDateTimeOffset(), source.RuntimeHash,
        source.ForceUpdate, MapDefinition(source.Definition), source.AdoptionWarnings.ToArray());
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

    private static SwarmServiceSpec? MapDefinition(HostingSwarmServiceSpec? source) => source is null
        ? null
        : new SwarmServiceSpec
        {
            Image = new SwarmExternalImage(Guid.Empty, source.Image),
            SchedulingMode = Parse(source.SchedulingMode, SwarmServiceSchedulingMode.Replicated),
            Replicas = source.Replicas,
            Command = source.Command,
            Arguments = source.Arguments,
            Environment = source.Environment,
            User = EmptyToNull(source.User),
            WorkingDirectory = EmptyToNull(source.WorkingDirectory),
            HealthCheck = source.HealthCheck is null ? null : new SwarmServiceHealthCheck(
                source.HealthCheck.Test,
                source.HealthCheck.IntervalNanoseconds,
                source.HealthCheck.TimeoutNanoseconds,
                source.HealthCheck.Retries,
                source.HealthCheck.StartPeriodNanoseconds),
            StopGracePeriodNanoseconds = source.StopGracePeriodNanoseconds,
            Ports = source.Ports.Select(static value => new SwarmServicePort(
                value.TargetPort,
                value.PublishedPort,
                value.Protocol,
                Parse(value.PublishMode, SwarmServicePortPublishMode.Ingress))).ToArray(),
            NetworkIds = source.NetworkIds,
            Mounts = source.Mounts.Select(static value => new SwarmServiceMount(
                Parse(value.Kind, SwarmServiceMountKind.Volume),
                value.Source,
                value.Target,
                value.ReadOnly)).ToArray(),
            Secrets = source.Secrets.Select(static value => new SwarmServiceSecretReference(
                value.Id, value.Name, value.TargetName)).ToArray(),
            Configs = source.Configs.Select(static value => new SwarmServiceConfigReference(
                value.Id, value.Name, value.TargetName)).ToArray(),
            Resources = source.Resources is null ? null : new SwarmServiceResources(
                source.Resources.LimitNanoCpus,
                source.Resources.LimitMemoryBytes,
                source.Resources.ReservationNanoCpus,
                source.Resources.ReservationMemoryBytes),
            PlacementConstraints = source.PlacementConstraints,
            RestartPolicy = source.RestartPolicy is null ? null : new SwarmServiceRestartPolicy(
                Parse(source.RestartPolicy.Condition, SwarmServiceRestartCondition.Any),
                source.RestartPolicy.DelayNanoseconds,
                source.RestartPolicy.MaximumAttempts,
                source.RestartPolicy.WindowNanoseconds),
            UpdatePolicy = source.UpdatePolicy is null ? null : new SwarmServiceUpdatePolicy(
                source.UpdatePolicy.Parallelism,
                source.UpdatePolicy.DelayNanoseconds,
                Parse(source.UpdatePolicy.Order, SwarmServiceUpdateOrder.StopFirst),
                Parse(source.UpdatePolicy.FailureAction, SwarmServiceUpdateFailureAction.Pause))
        };

    private static SwarmServiceSpec? MapDefinition(SwarmServiceMutationSpecMessage? source) => source is null
        ? null
        : new SwarmServiceSpec
        {
            Image = new SwarmExternalImage(Guid.Empty, source.Image),
            SchedulingMode = Parse(source.SchedulingMode, SwarmServiceSchedulingMode.Replicated),
            Replicas = source.HasReplicas ? source.Replicas : null,
            Command = source.Command.ToArray(),
            Arguments = source.Arguments.ToArray(),
            Environment = source.Environment.ToArray(),
            User = EmptyToNull(source.User),
            WorkingDirectory = EmptyToNull(source.WorkingDirectory),
            HealthCheck = source.HealthCheck is null ? null : new SwarmServiceHealthCheck(
                source.HealthCheck.Test.ToArray(),
                source.HealthCheck.HasIntervalNanoseconds ? source.HealthCheck.IntervalNanoseconds : null,
                source.HealthCheck.HasTimeoutNanoseconds ? source.HealthCheck.TimeoutNanoseconds : null,
                source.HealthCheck.HasRetries ? source.HealthCheck.Retries : null,
                source.HealthCheck.HasStartPeriodNanoseconds ? source.HealthCheck.StartPeriodNanoseconds : null),
            StopGracePeriodNanoseconds = source.HasStopGracePeriodNanoseconds ? source.StopGracePeriodNanoseconds : null,
            Ports = source.Ports.Select(static value => new SwarmServicePort(
                value.TargetPort,
                value.HasPublishedPort ? value.PublishedPort : null,
                value.Protocol,
                Parse(value.PublishMode, SwarmServicePortPublishMode.Ingress))).ToArray(),
            NetworkIds = source.NetworkIds.ToArray(),
            Mounts = source.Mounts.Select(static value => new SwarmServiceMount(
                Parse(value.Kind, SwarmServiceMountKind.Volume),
                value.Source,
                value.Target,
                value.ReadOnly)).ToArray(),
            Secrets = source.Secrets.Select(static value => new SwarmServiceSecretReference(
                value.Id, value.Name, value.TargetName)).ToArray(),
            Configs = source.Configs.Select(static value => new SwarmServiceConfigReference(
                value.Id, value.Name, value.TargetName)).ToArray(),
            Resources = source.Resources is null ? null : new SwarmServiceResources(
                source.Resources.HasLimitNanoCpus ? source.Resources.LimitNanoCpus : null,
                source.Resources.HasLimitMemoryBytes ? source.Resources.LimitMemoryBytes : null,
                source.Resources.HasReservationNanoCpus ? source.Resources.ReservationNanoCpus : null,
                source.Resources.HasReservationMemoryBytes ? source.Resources.ReservationMemoryBytes : null),
            PlacementConstraints = source.PlacementConstraints.ToArray(),
            RestartPolicy = source.RestartPolicy is null ? null : new SwarmServiceRestartPolicy(
                Parse(source.RestartPolicy.Condition, SwarmServiceRestartCondition.Any),
                source.RestartPolicy.HasDelayNanoseconds ? source.RestartPolicy.DelayNanoseconds : null,
                source.RestartPolicy.HasMaximumAttempts ? source.RestartPolicy.MaximumAttempts : null,
                source.RestartPolicy.HasWindowNanoseconds ? source.RestartPolicy.WindowNanoseconds : null),
            UpdatePolicy = source.UpdatePolicy is null ? null : new SwarmServiceUpdatePolicy(
                source.UpdatePolicy.Parallelism,
                source.UpdatePolicy.HasDelayNanoseconds ? source.UpdatePolicy.DelayNanoseconds : null,
                Parse(source.UpdatePolicy.Order, SwarmServiceUpdateOrder.StopFirst),
                Parse(source.UpdatePolicy.FailureAction, SwarmServiceUpdateFailureAction.Pause))
        };

    private static T Parse<T>(string? value, T fallback) where T : struct, Enum =>
        Enum.TryParse<T>(value, ignoreCase: true, out var parsed) ? parsed : fallback;
}
