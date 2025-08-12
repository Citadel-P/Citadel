using Domain;
using Domain.Entities;
using Domain.Entities.Platforms;

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
    PlatformDescriptor? PlatformDescriptor
    )
{
    internal static List<PlatformView> Map(IEnumerable<Platform> platforms)
        => [.. platforms.Select(Map)];

    internal static PlatformView Map(Platform platform)
        => platform.Map();
}

public sealed record PlatformsView(IEnumerable<PlatformView> Platforms)
{
    internal static PlatformsView Map(IEnumerable<Platform> platforms)
       => new(PlatformView.Map(platforms));
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