namespace Domain.Contracts.Resources.Platforms;

public sealed class PlatformStatsResult(
    long memTotal,
    long imageCount,
    int volumeCount,
    int networkCount,
    long containerCount,
    long containersPaused,
    long containersStopped,
    long containersRunning,
    DockerPlatformStat platformStat)
{
    internal PlatformStatsResult() : this(0,0,0,0,0,0,0,0, new DockerPlatformStat(0,0,0,0,0)) { }

    public long MemTotal { get; private set; } = memTotal;
    public long ImageCount { get; private set; } = imageCount;
    public int VolumeCount { get; private set; } = volumeCount;
    public int NetworkCount { get; private set; } = networkCount;
    public long ContainerCount { get; private set; } = containerCount;
    public long ContainersPaused { get; private set; } = containersPaused;
    public long ContainersStopped { get; private set; } = containersStopped;
    public long ContainersRunning { get; private set; } = containersRunning;
    public DockerPlatformStat PlatformStat { get; private set; } = platformStat;

    public void ReInitialize(
        long memTotal,
        long imageCount,
        int volumeCount,
        int networkCount,
        long containerCount,
        long containersPaused,
        long containersStopped,
        long containersRunning,
        DockerPlatformStat platformStat)
    {
        MemTotal = memTotal;
        ImageCount = imageCount;
        VolumeCount = volumeCount;
        NetworkCount = networkCount;
        ContainerCount = containerCount;
        ContainersPaused = containersPaused;
        ContainersStopped = containersStopped;
        ContainersRunning = containersRunning;

        PlatformStat.ReInitialize(
            created: platformStat.Created,
            memoryUsage: platformStat.MemoryUsage,
            cpuUsage: platformStat.CpuUsage,
            rxBytes: platformStat.RxBytes,
            txBytes: platformStat.TxBytes);
    }
}
