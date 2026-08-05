using Application.Permissions;
using Domain;
using Domain.Entities.Platforms;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

public sealed record SwarmNodeView(
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
    DateTimeOffset? UpdatedAt,
    DateTimeOffset ObservedAt,
    bool IsStale,
    PlatformCapabilities? Capabilities = null)
{
    public static SwarmNodeView Map(SwarmNodeProjection node) =>
        new(
            node.DockerNodeId,
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
            node.DockerCreatedAt,
            node.DockerUpdatedAt,
            node.ObservedAt,
            node.IsStale);

    public static async Task<SwarmNodeView> Map(
        SwarmNodeProjection node,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(node.PlatformId, ResourceType.Platform);
        return Map(node) with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
        };
    }
}

public sealed record SwarmNodesView(
    IReadOnlyList<SwarmNodeView> Items,
    PlatformCapabilities Capabilities)
{
    public static SwarmNodesView Map(IEnumerable<SwarmNodeProjection> nodes) =>
        new(nodes.Select(SwarmNodeView.Map).ToArray(), PlatformCapabilities.Empty);

    public static async Task<SwarmNodesView> Map(
        IEnumerable<SwarmNodeProjection> nodes,
        Guid platformId,
        IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToPlatformCapabilities(permissions);
        var items = nodes.Select(node => SwarmNodeView.Map(node) with { Capabilities = capabilities }).ToArray();
        return new SwarmNodesView(items, capabilities);
    }
}

public sealed record SwarmNodeInspectView(
    string Id, long VersionIndex, string Hostname, string Role, bool IsLeader,
    string Reachability, string Status, string? StatusMessage, string Availability,
    string EngineVersion, string OperatingSystem, string Architecture, string Address,
    IReadOnlyDictionary<string, string> Labels, int RunningTaskCount, int DesiredTaskCount,
    DateTimeOffset? CreatedAt, DateTimeOffset? UpdatedAt)
{
    public static SwarmNodeInspectView Map(Domain.Contracts.Resources.Swarm.SwarmNodeResult node) =>
        new(
            node.Id, node.VersionIndex, node.Hostname, node.Role, node.IsLeader,
            node.Reachability, node.Status, node.StatusMessage, node.Availability,
            node.EngineVersion, node.OperatingSystem, node.Architecture, node.Address,
            node.Labels, node.RunningTaskCount, node.DesiredTaskCount, node.CreatedAt, node.UpdatedAt);
}
