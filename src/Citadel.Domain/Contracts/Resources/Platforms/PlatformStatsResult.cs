namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformStatsResult(
    long MemTotal,
    long ImageCount,
    int VolumeCount,
    int NetworkCount,
    string AgentVersion,
    DockerPlatformStat PlatformStat,
    long? ImageUsedBytes = null,
    long? VolumeUsedBytes = null);
