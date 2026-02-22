namespace Domain.Entities.Platforms;

public record struct PlatformStat(
    long Created,
    double MemoryUsage,
    double CpuUsage,
    double RxBytes,
    double TxBytes,
    Guid? PlatformId = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
}