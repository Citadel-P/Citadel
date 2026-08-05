using Domain;
using Domain.Entities.Platforms;
using Application.Features.Swarm.Queries;
using Application.Permissions;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

public sealed record SwarmServiceView(
    string Id, long VersionIndex, string Name, string Mode, string Image,
    int RunningTaskCount, int DesiredTaskCount, string UpdateState, string? UpdateMessage,
    IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds, IReadOnlyList<string> SecretIds,
    IReadOnlyList<string> ConfigIds, IReadOnlyDictionary<string, string> Labels,
    SwarmServiceOwnership Ownership, string? DockerStackNamespace, string? OwnershipDiagnostic,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt, DateTimeOffset ObservedAt, bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmServiceView Map(SwarmServiceProjection value) => new(
        value.DockerServiceId, value.VersionIndex, value.Name, value.Mode, value.Image,
        value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
        value.Ports, value.NetworkIds, value.SecretIds, value.ConfigIds, value.Labels,
        value.Ownership, value.DockerStackNamespace, value.OwnershipDiagnostic,
        value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);

    public static async Task<SwarmServiceView> Map(
        SwarmServiceProjection value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(value.PlatformId, ResourceType.Platform);
        return Map(value) with { Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions) };
    }
}
public sealed record SwarmServicesView(
    IReadOnlyList<SwarmServiceView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmServicesView Map(IEnumerable<SwarmServiceProjection> values) =>
        new(values.Select(SwarmServiceView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmServicesView> Map(
        IEnumerable<SwarmServiceProjection> values,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = values.Select(value => SwarmServiceView.Map(value) with { Capabilities = capabilities }).ToArray();
        return new SwarmServicesView(items, capabilities);
    }
}

public sealed record SwarmServiceInspectView(
    string Id, long VersionIndex, string Name, string Mode, string Image,
    int RunningTaskCount, int DesiredTaskCount, string UpdateState, string? UpdateMessage,
    IReadOnlyList<string> Ports, IReadOnlyList<string> NetworkIds, IReadOnlyList<string> SecretIds,
    IReadOnlyList<string> ConfigIds, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt)
{
    public static SwarmServiceInspectView Map(Domain.Contracts.Resources.Swarm.SwarmServiceResult value) => new(
        value.Id, value.VersionIndex, value.Name, value.Mode, value.Image,
        value.RunningTaskCount, value.DesiredTaskCount, value.UpdateState, value.UpdateMessage,
        value.Ports, value.NetworkIds, value.SecretIds, value.ConfigIds, value.Labels,
        value.CreatedAt, value.UpdatedAt);
}

public sealed record SwarmTaskView(
    string Id, long VersionIndex, string Name, string ServiceId, string ServiceName, int? Slot,
    string NodeId, string NodeHostname, string DesiredState, string State, string? StatusMessage,
    string? Error, string Image, IReadOnlyList<string> Ports, DateTimeOffset? StatusTimestamp,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt, DateTimeOffset ObservedAt, bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmTaskView Map(SwarmTaskProjection value) => new(
        value.DockerTaskId, value.VersionIndex, value.Name, value.DockerServiceId, value.ServiceName,
        value.Slot, value.DockerNodeId, value.NodeHostname, value.DesiredState, value.State,
        value.StatusMessage, value.Error, value.Image, value.Ports, value.StatusTimestamp,
        value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);

    public static async Task<SwarmTaskView> Map(
        SwarmTaskProjection value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(value.PlatformId, ResourceType.Platform);
        return Map(value) with { Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions) };
    }
}
public sealed record SwarmTasksView(
    IReadOnlyList<SwarmTaskView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmTasksView Map(IEnumerable<SwarmTaskProjection> values) =>
        new(values.Select(SwarmTaskView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmTasksView> Map(
        IEnumerable<SwarmTaskProjection> values,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = values.Select(value => SwarmTaskView.Map(value) with { Capabilities = capabilities }).ToArray();
        return new SwarmTasksView(items, capabilities);
    }
}

public sealed record SwarmTaskInspectView(
    string Id, long VersionIndex, string Name, string ServiceId, int? Slot, string NodeId,
    string DesiredState, string State, string? StatusMessage, string? Error, string Image,
    IReadOnlyList<string> Ports, DateTimeOffset? StatusTimestamp, DateTimeOffset? CreatedAt,
    DateTimeOffset? UpdatedAt, string? ContainerId)
{
    public static SwarmTaskInspectView Map(Domain.Contracts.Resources.Swarm.SwarmTaskResult value) => new(
        value.Id, value.VersionIndex, value.Name, value.ServiceId, value.Slot, value.NodeId,
        value.DesiredState, value.State, value.StatusMessage, value.Error, value.Image,
        value.Ports, value.StatusTimestamp, value.CreatedAt, value.UpdatedAt, value.ContainerId);
}

public sealed record SwarmTaskStatsView(
    string DockerContainerId,
    IReadOnlyList<ContainerStatView> Stats)
{
    public static SwarmTaskStatsView Map(Domain.Contracts.Resources.Swarm.SwarmTaskStatsResult value) =>
        new(value.DockerContainerId, ContainerStatView.Map(value.Stats));
}

public sealed record SwarmNetworkView(
    string Id, string Name, string Scope, string Driver, bool IsAttachable, bool IsInternal,
    bool IsIngress, bool IsEncrypted, bool EnableIPv6, IReadOnlyList<string> Subnets,
    IReadOnlyList<string> ServiceNames, IReadOnlyDictionary<string, string> Labels,
    DateTimeOffset? CreatedAt, DateTimeOffset ObservedAt, bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmNetworkView Map(SwarmNetworkProjection value) => new(
        value.DockerNetworkId, value.Name, value.Scope, value.Driver, value.IsAttachable,
        value.IsInternal, value.IsIngress, value.IsEncrypted, value.EnableIPv6, value.Subnets,
        value.ServiceNames, value.Labels, value.DockerCreatedAt, value.ObservedAt, value.IsStale);

    public static async Task<SwarmNetworkView> Map(
        SwarmNetworkProjection value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(value.PlatformId, ResourceType.Platform);
        return Map(value) with { Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions) };
    }
}
public sealed record SwarmNetworksView(
    IReadOnlyList<SwarmNetworkView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmNetworksView Map(IEnumerable<SwarmNetworkProjection> values) =>
        new(values.Select(SwarmNetworkView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmNetworksView> Map(
        IEnumerable<SwarmNetworkProjection> values,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = values.Select(value => SwarmNetworkView.Map(value) with { Capabilities = capabilities }).ToArray();
        return new SwarmNetworksView(items, capabilities);
    }
}

public sealed record SwarmSecretView(
    string Id, long VersionIndex, string Name, string? Driver, IReadOnlyList<string> ServiceNames,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmSecretView Map(SwarmSecretProjection value) => new(
        value.DockerSecretId, value.VersionIndex, value.Name, value.Driver, value.ServiceNames,
        value.Labels, value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);

    public static async Task<SwarmSecretView> Map(
        SwarmSecretProjection value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(value.PlatformId, ResourceType.Platform);
        return Map(value) with { Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions) };
    }
}
public sealed record SwarmSecretsView(
    IReadOnlyList<SwarmSecretView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmSecretsView Map(IEnumerable<SwarmSecretProjection> values) =>
        new(values.Select(SwarmSecretView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmSecretsView> Map(
        IEnumerable<SwarmSecretProjection> values,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = values.Select(value => SwarmSecretView.Map(value) with { Capabilities = capabilities }).ToArray();
        return new SwarmSecretsView(items, capabilities);
    }
}

public sealed record SwarmConfigView(
    string Id, long VersionIndex, string Name, string? TemplatingDriver, IReadOnlyList<string> ServiceNames,
    IReadOnlyDictionary<string, string> Labels, DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt, bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmConfigView Map(SwarmConfigProjection value) => new(
        value.DockerConfigId, value.VersionIndex, value.Name, value.TemplatingDriver, value.ServiceNames,
        value.Labels, value.DockerCreatedAt, value.DockerUpdatedAt, value.ObservedAt, value.IsStale);

    public static async Task<SwarmConfigView> Map(
        SwarmConfigProjection value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(value.PlatformId, ResourceType.Platform);
        return Map(value) with { Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions) };
    }
}
public sealed record SwarmConfigsView(
    IReadOnlyList<SwarmConfigView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmConfigsView Map(IEnumerable<SwarmConfigProjection> values) =>
        new(values.Select(SwarmConfigView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmConfigsView> Map(
        IEnumerable<SwarmConfigProjection> values,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = values.Select(value => SwarmConfigView.Map(value) with { Capabilities = capabilities }).ToArray();
        return new SwarmConfigsView(items, capabilities);
    }
}

public sealed record SwarmLogsView(IReadOnlyList<string> Lines, bool Truncated)
{
    public static SwarmLogsView Map(Domain.Contracts.Resources.Swarm.SwarmLogsResult value) =>
        new(value.Lines, value.Truncated);
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

public sealed record SwarmOverviewView(
    Guid PlatformId,
    string Health,
    string? Message,
    bool IsStale,
    int NodeCount,
    int ManagerCount,
    int ServiceCount,
    int RunningTaskCount,
    int DesiredTaskCount,
    int NetworkCount,
    PlatformCapabilities Capabilities)
{
    public static SwarmOverviewView Map(Guid platformId, SwarmOverviewResult value)
    {
        var summary = value.Summary;
        var stale = summary.IsStale;
        var health = value.ConnectionStatus != PlatformStatus.Online
            ? "Offline"
            : stale
                ? "Stale"
                : !value.ControlAvailable || !string.IsNullOrWhiteSpace(value.Error)
                    ? "Degraded"
                    : "Healthy";
        var message = health switch
        {
            "Offline" => "The Swarm manager is offline. Last-known inventory remains available.",
            "Stale" => "The latest inventory refresh failed. Last-known inventory may be out of date.",
            "Degraded" => value.Error ?? "The connected Docker node does not currently expose Swarm manager control.",
            _ => null
        };

        return new SwarmOverviewView(
            platformId,
            health,
            message,
            stale,
            summary.NodeCount,
            summary.ManagerCount,
            summary.ServiceCount,
            summary.RunningTaskCount,
            summary.DesiredTaskCount,
            summary.NetworkCount,
            PlatformCapabilities.Empty);
    }

    public static async Task<SwarmOverviewView> Map(
        Guid platformId,
        SwarmOverviewResult value,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        return Map(platformId, value) with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
        };
    }
}
