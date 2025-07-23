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

public class DockerContainerStat(double? memoryUsage, double? cpuUsage, double? memoryLimit, double? rxBytes, double? txBytes)
{
    public double? MemoryUsage { get; private set; } = memoryUsage;
    public double? CpuUsage { get; private set; } = cpuUsage;
    public double? MemoryLimit { get; private set; } = memoryLimit;
    public double? RxBytes { get; private set; } = rxBytes;
    public double? TxBytes { get; private set; } = txBytes;

    public DockerContainerStat() : this(0, 0, 0, 0, 0) { }

    public void ReInitialize(
        double? memoryUsage,
        double? cpuUsage,
        double? memoryLimit,
        double? rxBytes,
        double? txBytes)
    {
        MemoryUsage = memoryUsage;
        CpuUsage = cpuUsage;
        MemoryLimit = memoryLimit;
        RxBytes = rxBytes;
        TxBytes = txBytes;
    }
}