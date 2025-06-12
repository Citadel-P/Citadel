namespace Domain.Entities;

public class ContainerStat
{
    private ContainerStat() { /* For EF Core */ }

    public ContainerStat(
        Guid containerId,
        double? memoryUsage,
        double? cpuUsage,
        double? memoryLimit,
        double? rxBytes,
        double? txBytes,
        long? created)
    {
        Id = Guid.CreateVersion7();
        Created = created is not null ? created.Value : DateTimeOffset.UtcNow.Ticks;
        MemoryUsage = memoryUsage;
        CpuUsage = cpuUsage;
        MemoryLimit = memoryLimit;
        ContainerId = containerId;
        RxBytes = rxBytes;
        TxBytes = txBytes;
    }
    public Guid Id { get; private set; }
    public Guid ContainerId { get; private set; }
    public long Created { get; private set; }
    public double? MemoryUsage { get; private set; }
    public double? CpuUsage { get; private set; }
    public double? MemoryLimit { get; private set; }
    public double? RxBytes { get; private set; }
    public double? TxBytes { get; private set; }

    public void PartialUpdate(
        Guid? containerId,
        long? created)
    {
        if (containerId != null) ContainerId = containerId.Value;
        if (created != null) Created = created.Value;
    }
}
