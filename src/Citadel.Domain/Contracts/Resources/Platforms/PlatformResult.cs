using Domain.Entities.Platforms;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PlatformResult(
        string Name,
        string Address,
        int NetworkCount,
        int VolumeCount,
        long ImageCount,
        long CpuCount,
        long MemTotal,
        string? ServerVersion,
        string? AgentVersion,
        PlatformDescriptor? Descriptor,
        DockerPlatformStat? PlatformStat = null,
        string? AgentRuntimeImage = null);

public sealed class DockerPlatformStat(
        long created,
        double memoryUsage,
        double cpuUsage,
        double rxBytes,
        double txBytes,
        long containerCount,
        long containersPaused,
        long containersStopped,
        long containersRunning,
        long? diskUsedBytes = null,
        long? diskTotalBytes = null,
        double? diskUsage = null)
{
    public long Created { get; private set; } = created;
    public double MemoryUsage { get; private set; } = memoryUsage;
    public double CpuUsage { get; private set; } = cpuUsage;
    public double RxBytes { get; private set; } = rxBytes;
    public double TxBytes { get; private set; } = txBytes;
    public long ContainerCount { get; private set; } = containerCount;
    public long ContainersPaused { get; private set; } = containersPaused;
    public long ContainersStopped { get; private set; } = containersStopped;
    public long ContainersRunning { get; private set; } = containersRunning;
    public long? DiskUsedBytes { get; private set; } = diskUsedBytes;
    public long? DiskTotalBytes { get; private set; } = diskTotalBytes;
    public double? DiskUsage { get; private set; } = diskUsage;
    public void ReInitialize(
        long created, 
        double memoryUsage, 
        double cpuUsage, 
        double rxBytes,
        double txBytes,
        long containerCount,
        long containersPaused,
        long containersStopped,
        long containersRunning,
        long? diskUsedBytes = null,
        long? diskTotalBytes = null,
        double? diskUsage = null
        )
    {
        Created = created;
        MemoryUsage = memoryUsage;
        CpuUsage = cpuUsage;
        RxBytes = rxBytes;
        TxBytes = txBytes;
        ContainerCount = containerCount;
        ContainersPaused = containersPaused;
        ContainersStopped = containersStopped;
        ContainersRunning = containersRunning;
        DiskUsedBytes = diskUsedBytes;
        DiskTotalBytes = diskTotalBytes;
        DiskUsage = diskUsage;
    }
}
