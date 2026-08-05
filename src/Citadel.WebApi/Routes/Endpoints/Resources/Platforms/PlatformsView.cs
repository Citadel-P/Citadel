using Application.Permissions;
using Domain;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformView(
    Guid Id,
    string Name,
    string? Description,
    string Address,
    int NetworkCount,
    int VolumeCount,
    long ImageCount,
    long CpuCount,
    long MemTotal,
    string? AgentVersion,
    string? ServerVersion,
    PlatformType Type,
    PlatformStatus Status,
    PlatformConnectorType ConnectorType,
    long DeploymentCount,
    long StackCount,
    PlatformWorkloadStatusCountsView DeploymentStatusCounts,
    PlatformWorkloadStatusCountsView StackStatusCounts,
    IEnumerable<PlatformStatView>? Stats,
    PlatformDescriptor? PlatformDescriptor,
    string? ClusterId,
    IReadOnlyList<TagSummaryView> Tags = null!,
    PlatformCapabilities? Capabilities = null,
    SwarmCapabilities? SwarmCapabilities = null
    )
{
    internal static List<PlatformView> Map(IEnumerable<Platform> platforms)
            => [.. platforms.Select(Map)];

    internal static PlatformView Map(Platform platform)
        => platform.Map();

    internal static async Task<PlatformView[]> Map(IEnumerable<Platform> platforms, IPermissionEvaluator permissionEvaluator)
    {
        var list = platforms as Platform[] ?? [.. platforms];

        if (list.Length == 0)
            return [];

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator
            .EvaluateAsync(ids, ResourceType.Platform);

        var views = new PlatformView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var platform = list[i];

            var baseView = Map(platform);

            perms.TryGetValue(platform.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToPlatformCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta),
                SwarmCapabilities = CapabilityMapper.ToSwarmCapabilities(
                    platform,
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return views;
    }

    internal static async Task<PlatformView> Map(Platform platform, IPermissionEvaluator permissionService)
    {
        var permissions = await permissionService.EvaluateAsync(platform.Id, ResourceType.Platform);
        return platform.Map() with
        {
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions),
            SwarmCapabilities = CapabilityMapper.ToSwarmCapabilities(platform, permissions)
        };
    }
}

public sealed record PlatformsView(IEnumerable<PlatformView> Platforms, ResourceCapabilities Capabilities)
{
    internal static async Task<PlatformsView> Map(IEnumerable<Platform> platforms, IPermissionEvaluator permissionEvaluator)
    {
        var platformViews = await PlatformView.Map(platforms, permissionEvaluator);
        var capabilities = CapabilityMapper.ToResourceCapabilities(
            await permissionEvaluator.EvaluateAsync(ResourceType.Platform));
        return new PlatformsView(platformViews, capabilities);
    }
}

internal static class PlatformMapperExtension
{
    internal static PlatformView Map(this Platform platform) => new(
        Id: platform.Id,
        Name: platform.Name,
        Description: platform.Description,
        Address: platform.Address,
        Status: platform.Status,
        Type: GetPlatformType(platform.PlatformDescriptor),
        ConnectorType: platform.ConnectorType,
        DeploymentCount: platform.DeploymentCount,
        StackCount: platform.StackCount,
        DeploymentStatusCounts: PlatformWorkloadStatusCountsView.Map(platform.DeploymentStatusCounts),
        StackStatusCounts: PlatformWorkloadStatusCountsView.Map(platform.StackStatusCounts),
        NetworkCount: platform.NetworkCount,
        VolumeCount: platform.VolumeCount,
        ImageCount: platform.ImageCount,
        CpuCount: platform.CpuCount,
        MemTotal: platform.MemTotal,
        ServerVersion: platform.ServerVersion,
        AgentVersion: platform.AgentVersion,
        PlatformDescriptor: platform.PlatformDescriptor,
        ClusterId: platform.ClusterId,
        Stats: platform.Stats?.Select(Map)?.ToList(),
        Tags: [.. platform.Tags.Select(TagSummaryView.Map)]);

    private static PlatformType GetPlatformType(PlatformDescriptor platformDescriptor) =>
        platformDescriptor switch
        {
            DockerSwarmPlatformDescriptor => PlatformType.DockerSwarm,
            KubernetesPlatformDescriptor => PlatformType.Kubernetes,
            DockerPlatformDescriptor => PlatformType.Docker,
            _ => PlatformType.Docker
        };

    internal static PlatformStatView Map(this PlatformStat stat) => new (
            MemoryUsage: stat.MemoryUsage,
            CpuUsage: stat.CpuUsage,
            Created: stat.Created,
            RxBytes: stat.RxBytes,
            TxBytes: stat.TxBytes,
            DiskUsedBytes: stat.DiskUsedBytes,
            DiskTotalBytes: stat.DiskTotalBytes,
            DiskUsage: stat.DiskUsage);
}

public sealed record PlatformWorkloadStatusCountsView(
    long Total,
    long Healthy,
    long Degraded,
    long Failed,
    long Stopped,
    long Paused,
    long InProgress,
    long Unknown)
{
    internal static PlatformWorkloadStatusCountsView Map(PlatformWorkloadStatusCounts counts) => new(
        Total: counts.Total,
        Healthy: counts.Healthy,
        Degraded: counts.Degraded,
        Failed: counts.Failed,
        Stopped: counts.Stopped,
        Paused: counts.Paused,
        InProgress: counts.InProgress,
        Unknown: counts.Unknown);
}
