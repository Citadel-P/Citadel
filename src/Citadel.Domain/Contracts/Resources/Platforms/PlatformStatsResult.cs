namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformStatsResult(
    long MemTotal,
    long ImageCount,
    int VolumeCount,
    int NetworkCount,
    DockerPlatformStat PlatformStat);
