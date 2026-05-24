using Application.Permissions;
using Domain;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformView(
    Guid Id,
    string Name,
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
    IEnumerable<PlatformStatView>? Stats,
    PlatformDescriptor? PlatformDescriptor,
     PlatformCapabilities? Capabilities = null
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
            Capabilities = CapabilityMapper.ToPlatformCapabilities(permissions)
        };
    }
}

public sealed record PlatformsView(IEnumerable<PlatformView> Platforms)
{
    internal static async Task<PlatformsView> Map(IEnumerable<Platform> platforms, IPermissionEvaluator permissionEvaluator)
       => new(await PlatformView.Map(platforms, permissionEvaluator));
}

internal static class PlatformMapperExtension
{
    internal static PlatformView Map(this Platform platform) => new(
        Id: platform.Id,
        Name: platform.Name,
        Address: platform.Address,
        Status: platform.Status,
        Type: GetPlatformType(platform.PlatformDescriptor),
        ConnectorType: platform.ConnectorType,
        NetworkCount: platform.NetworkCount,
        VolumeCount: platform.VolumeCount,
        ImageCount: platform.ImageCount,
        CpuCount: platform.CpuCount,
        MemTotal: platform.MemTotal,
        ServerVersion: platform.ServerVersion,
        AgentVersion: platform.AgentVersion,
        PlatformDescriptor: platform.PlatformDescriptor,
        Stats: platform.Stats?.Select(Map)?.ToList());

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
            TxBytes: stat.TxBytes);
}