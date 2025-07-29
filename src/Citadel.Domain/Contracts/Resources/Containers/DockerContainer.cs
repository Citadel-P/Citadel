using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record DockerContainer
    (
    string Name,
    string Image,
    string ContainerId,
    ContainerStateStatus State,
    long? Created = null,
    string? Stack = null,
    string? Command = null,
    DockerContainerStat? ContainerStat = null,
    IReadOnlyList<ContainerPort>? Ports = null
    );

public class DockerContainerStat(double? memoryActive, double? memoryCache, double? cpuUsage, double? memoryLimit, double? rxBytes, double? txBytes)
{
    public double? MemoryActive { get; private set; } = memoryActive;
    public double? MemoryCache { get; private set; } = memoryCache;
    public double? CpuUsage { get; private set; } = cpuUsage;
    public double? MemoryLimit { get; private set; } = memoryLimit;
    public double? RxBytes { get; private set; } = rxBytes;
    public double? TxBytes { get; private set; } = txBytes;

    internal DockerContainerStat() : this(0, 0, 0, 0, 0, 0) { }

    public void ReInitialize(
        double? memoryActive,
        double? memoryCache,
        double? cpuUsage,
        double? memoryLimit,
        double? rxBytes,
        double? txBytes)
    {
        MemoryActive = memoryActive;
        MemoryCache = memoryCache;
        CpuUsage = cpuUsage;
        MemoryLimit = memoryLimit;
        RxBytes = rxBytes;
        TxBytes = txBytes;
    }
}