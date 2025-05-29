using Infrastructure.TaskJobs;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformStatsBatchView(
    Guid PlatformId,
    int NetworksCount,
    int VolumesCount,
    long Containers,
    long ContainersRunning,
    long ContainersPaused,
    long ContainersStopped,
    long Images,
    long MemTotal,
    PlatformStatView Stat)
{
    internal static PlatformStatsBatchView Map(PlatformStatsBatch platform)
    {
        return new PlatformStatsBatchView(
            PlatformId: platform.PlatformId,
            NetworksCount: platform.NetworksCount,
            VolumesCount: platform.VolumesCount,
            Containers: platform.Containers,
            ContainersRunning: platform.ContainersRunning,
            ContainersPaused: platform.ContainersPaused,
            ContainersStopped: platform.ContainersStopped,
            Images: platform.Images,
            MemTotal: platform.MemTotal,
            Stat: platform.Stat.Map());
    }
}