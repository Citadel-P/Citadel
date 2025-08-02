using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed class DockerContainer
    (
    string name,
    string image,
    string containerId,
    ContainerStateStatus state,
    long? created = null,
    string? stack = null,
    DockerContainerStat? containerStat = null,
    IReadOnlyList<ContainerPort>? ports = null
    )
{
    public string Name { get; private set; } = name;
    public string Image { get; private set; } = image;
    public string ContainerId { get; private set; } = containerId;
    public ContainerStateStatus State { get; private set; } = state;
    public long? Created { get; private set; } = created;
    public string? Stack { get; private set; } = stack;
    public DockerContainerStat? ContainerStat { get; private set; } = containerStat;
    public IReadOnlyList<ContainerPort>? Ports { get; private set; } = ports;
    internal DockerContainer() : this(string.Empty, string.Empty, string.Empty, ContainerStateStatus.Unknown, containerStat: new DockerContainerStat()) { } // For pooled object usage
    public void ReInitialize(
        string name,
        string image,
        string containerId,
        ContainerStateStatus state,
        long? created,
        string? stack,
        DockerContainerStat? containerStat,
        IReadOnlyList<ContainerPort>? ports)
    {
        Name = name;
        Image = image;
        ContainerId = containerId;
        State = state;
        Created = created;
        Stack = stack;
        ContainerStat = containerStat;
        Ports = ports ?? [];
    }
}

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