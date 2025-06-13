using Domain.Entities;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformStatsBatch(
    Guid PlatformId,
    long MemTotal,
    long ImageCount,
    int VolumeCount,
    int NetworkCount,
    long ContainerCount,
    long ContainersPaused,
    long ContainersStopped,
    long ContainersRunning,
    PlatformStat PlatformStat);
