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
    internal static PlatformStatsBatchView Map(PlatformStatsBatch platform)
    {
        return new PlatformStatsBatchView(
            ImageCount: platform.ImageCount,
            MemTotal: platform.MemTotal,
            PlatformId: platform.PlatformId,
            VolumeCount: platform.VolumeCount,
            ContainerCount: platform.ContainerCount,
            NetworkCount: platform.NetworkCount,
            ContainersRunning: platform.ContainersRunning,
            ContainersPaused: platform.ContainersPaused,
            ContainersStopped: platform.ContainersStopped,
            Stat: platform.PlatformStat.Map());
    }
}