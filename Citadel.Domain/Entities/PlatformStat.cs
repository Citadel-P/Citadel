using System.Text.Json.Serialization;

namespace Domain.Entities;

[method: JsonConstructor]
public class PlatformStat(
    long created,
    double memoryUsage,
    double cpuUsage,
    double rxBytes,
    double txBytes,
    Guid? platformId = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId ?? Guid.Empty;
    public long Created { get; private set; } = created;
    public double MemoryUsage { get; private set; } = memoryUsage;
    public double CpuUsage { get; private set; } = cpuUsage;
    public double RxBytes { get; private set; } = rxBytes;
    public double TxBytes { get; private set; } = txBytes;

    public static PlatformStat FromPersistence(
        Guid id,
        long created,
        double memoryUsage = 0.0,
        double cpuUsage = 0.0,
        double rxBytes = 0.0,
        double txBytes = 0.0,
        Guid? platformId = null)
    {
        return new PlatformStat(
            created: created,
            memoryUsage: memoryUsage,
            cpuUsage: cpuUsage,
            rxBytes: rxBytes,
            txBytes: txBytes,
            platformId: platformId)
        {
            Id = id
        };
    }
}