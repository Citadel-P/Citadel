namespace Domain.Contracts.Resources.Platforms;

public sealed class PlatformStatsResult(
    long memTotal,
    long imageCount,
    int volumeCount,
    int networkCount,
    DockerPlatformStat platformStat)
{
    internal PlatformStatsResult() : this(0,0,0,0, new DockerPlatformStat(0,0,0,0,0,0,0,0,0)) { }

    public long MemTotal { get; private set; } = memTotal;
    public long ImageCount { get; private set; } = imageCount;
    public int VolumeCount { get; private set; } = volumeCount;
    public int NetworkCount { get; private set; } = networkCount;
    public DockerPlatformStat PlatformStat { get; private set; } = platformStat;

    public void ReInitialize(
        long memTotal,
        long imageCount,
        int volumeCount,
        int networkCount,
        DockerPlatformStat platformStat)
    {
        MemTotal = memTotal;
        ImageCount = imageCount;
        VolumeCount = volumeCount;
        NetworkCount = networkCount;

        PlatformStat.ReInitialize(
            created: platformStat.Created,
            memoryUsage: platformStat.MemoryUsage,
            cpuUsage: platformStat.CpuUsage,
            rxBytes: platformStat.RxBytes,
            txBytes: platformStat.TxBytes,
            containerCount: platformStat.ContainerCount,
            containersPaused: platformStat.ContainersPaused,
            containersStopped: platformStat.ContainersStopped,
            containersRunning: platformStat.ContainersRunning);
    }
    
}
