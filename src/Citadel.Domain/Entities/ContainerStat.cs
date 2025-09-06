namespace Domain.Entities;

public record struct ContainerStat(
    Guid ContainerId,
    double? MemoryActive,
    double? MemoryCache,
    double? CpuUsage,
    double? MemoryLimit,
    double? RxBytes,
    double? TxBytes,
    long? Created = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
}
