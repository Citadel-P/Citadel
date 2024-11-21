namespace Infrastructure.Entities;

public class PlatformStat
{
    public Guid Id { get; private set; }

    public Guid PlatformId { get; private set; }

    /// <summary>
    /// Total memory usage (%)
    /// </summary>
    public double MemoryUsage { get; private set; }

    /// <summary>
    /// Total cpu usage (%)
    /// </summary>
    public double CpuUsage { get; private set; }

    /// <summary>
    /// Snapshot time in utc
    /// </summary>
    public DateTime CreatedAtUtc { get; private set; }

    /// <summary>
    /// Create a new platform stat
    /// </summary>
    public static PlatformStat Create(Guid platformId, double memoryUsage, double cpuUsage, DateTime createdAtUtc)
    {
        return new PlatformStat()
        {
            Id = Guid.CreateVersion7(),
            PlatformId = platformId,
            MemoryUsage = memoryUsage,
            CpuUsage = cpuUsage,
            CreatedAtUtc = createdAtUtc
        };
    }
}