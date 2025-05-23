namespace Infrastructure.Entities;

public class ContainerStat
{
    public Guid Id { get; private set; }
    public Guid ContainerInfoId { get; private set; }
    public long Created { get; private set; }
    public double? MemoryUsage { get; private set; }
    public double? CpuUsage { get; private set; }
    public double? MemoryLimit { get; private set; }
    public double? RxBytes { get; private set; }
    public double? TxBytes { get; private set; }

    public static ContainerStat Create(
        Guid containerInfoId,
        double? memoryUsage,
        double? cpuUsage,
        double? memoryLimit,
        double? rxBytes,
        double? txBytes,
        long? created
        ) => new()
        {
            Id = Guid.CreateVersion7(),
            Created = created is not null ? created.Value : DateTimeOffset.UtcNow.Ticks,
            MemoryUsage = memoryUsage,
            CpuUsage = cpuUsage,
            MemoryLimit = memoryLimit,
            ContainerInfoId = containerInfoId,
            RxBytes = rxBytes,
            TxBytes = txBytes
        };
}
