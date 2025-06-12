using System.Text.Json.Serialization;

namespace Domain.Entities;

public class PlatformStat
{
    private PlatformStat() { /* For EF Core */ }

    [method: JsonConstructor]
    public PlatformStat(
        long created,
        double memoryUsage,
        double cpuUsage,
        double rxBytes,
        double txBytes,
        Guid? platformId = null)
    {
        Id = Guid.CreateVersion7();
        Created = created;
        MemoryUsage = memoryUsage;
        CpuUsage = cpuUsage;
        RxBytes = rxBytes;
        TxBytes = txBytes;
        PlatformId = platformId ?? Guid.Empty;
    }

    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public long Created { get; private set; }
    public double MemoryUsage { get; private set; }
    public double CpuUsage { get; private set; }
    public double RxBytes { get; private set; }
    public double TxBytes { get; private set; }
}