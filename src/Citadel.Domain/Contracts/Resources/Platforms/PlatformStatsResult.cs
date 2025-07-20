namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformStatsResult(
    long MemTotal,
    long ImageCount,
    int VolumeCount,
    int NetworkCount,
    long ContainerCount,
    long ContainersPaused,
    long ContainersStopped,
    long ContainersRunning,
    DockerPlatformStat PlatformStat);
