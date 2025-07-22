namespace Domain.Entities;

public class ContainerStat(
    Guid containerId,
    double? memoryUsage,
    double? cpuUsage,
    double? memoryLimit,
    double? rxBytes,
    double? txBytes,
    long? created = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid ContainerId { get; private set; } = containerId;
    public long Created { get; private set; } = created is not null ? created.Value : DateTimeOffset.UtcNow.Ticks;
    public double? MemoryUsage { get; private set; } = memoryUsage;
    public double? CpuUsage { get; private set; } = cpuUsage;
    public double? MemoryLimit { get; private set; } = memoryLimit;
    public double? RxBytes { get; private set; } = rxBytes;
    public double? TxBytes { get; private set; } = txBytes;

    public ContainerStat() : this(Guid.Empty, 0, 0, 0, 0, 0) { }

    public void ReInitialize(
        Guid containerId,
        long created,
        double? memoryUsage,
        double? cpuUsage,
        double? memoryLimit,
        double? rxBytes,
        double? txBytes)
    {
        Id = Guid.CreateVersion7();
        ContainerId = containerId;
        Created = created;
        MemoryUsage = memoryUsage;
        CpuUsage = cpuUsage;
        MemoryLimit = memoryLimit;
        RxBytes = rxBytes;
        TxBytes = txBytes;
    }

    public static ContainerStat FromPersistence(
        Guid id,
        Guid containerId,
        long created,
        double? memoryUsage = null,
        double? cpuUsage = null,
        double? memoryLimit = null,
        double? rxBytes = null,
        double? txBytes = null)
    {
        return new ContainerStat(
            containerId: containerId,
            memoryUsage: memoryUsage,
            cpuUsage: cpuUsage,
            memoryLimit: memoryLimit,
            rxBytes: rxBytes,
            txBytes: txBytes,
            created: created)
        {
            Id = id
        };
    }

    public void PartialUpdate(
        Guid? containerId,
        long? created)
    {
        if (containerId != null) ContainerId = containerId.Value;
        if (created != null) Created = created.Value;
    }
}
