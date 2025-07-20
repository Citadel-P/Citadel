using Domain.Contracts.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformStatsBatchView(
    Guid PlatformId,
    long MemTotal,
    long ImageCount,
    int VolumeCount,
    int NetworkCount,
    long ContainerCount,
    long ContainersPaused,
    long ContainersRunning,
    long ContainersStopped,
    PlatformStatView Stat)
{
    internal static PlatformStatsBatchView Map(Guid platformId,PlatformStatsResult platform)
    {
        return new PlatformStatsBatchView(
            PlatformId: platformId,
            MemTotal: platform.MemTotal,
            ImageCount: platform.ImageCount,
            VolumeCount: platform.VolumeCount,
            ContainerCount: platform.ContainerCount,
            NetworkCount: platform.NetworkCount,
            ContainersRunning: platform.ContainersRunning,
            ContainersPaused: platform.ContainersPaused,
            ContainersStopped: platform.ContainersStopped,
            Stat: Map(platform.PlatformStat));
    }

    internal static PlatformStatView Map(DockerPlatformStat stat) => new(
            MemoryUsage: stat?.MemoryUsage ?? 0,
            CpuUsage: stat?.CpuUsage ?? 0,
            Created: stat?.Created ?? 0,
            RxBytes: stat?.RxBytes ?? 0,
            TxBytes: stat?.TxBytes ?? 0);
}