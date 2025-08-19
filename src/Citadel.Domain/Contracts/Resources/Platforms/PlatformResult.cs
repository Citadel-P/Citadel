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
        DockerPlatformStat? PlatformStat = null);

public sealed class DockerPlatformStat(
        long created,
        double memoryUsage,
        double cpuUsage,
        double rxBytes,
        double txBytes,
        long containerCount,
        long containersPaused,
        long containersStopped,
        long containersRunning)
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
    public void ReInitialize(
        long created, 
        double memoryUsage, 
        double cpuUsage, 
        double rxBytes,
        double txBytes,
        long containerCount,
        long containersPaused,
        long containersStopped,
        long containersRunning
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
    }
}