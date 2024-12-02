namespace Infrastructure.Entities;

public class PlatformStat
{
    public Guid Id { get; private set; }

    public Guid PlatformId { get; private set; }

    /// <summary>
    /// Snapshot time in utc
    /// </summary>
    public long Created { get; private set; }

    /// <summary>
    /// Total memory usage (%)
    /// </summary>
    public double MemoryUsage { get; private set; }

    /// <summary>
    /// Total cpu usage (%)
    /// </summary>
    public double CpuUsage { get; private set; }

    /// <summary>
    /// Received Bytes
    /// </summary>
    public double RxBytes { get; private set; }

    /// <summary>
    /// Transferred Bytes
    /// </summary>
    public double TxBytes { get; private set; }

    /// <summary>
    /// Create a new platform stat
    /// </summary>
    public static PlatformStat Create(
        long created,
        double memoryUsage,
        double cpuUsage,
        double rxBytes,
        double txBytes,
        Guid? platformId = null) => new ()
        {
            Id = Guid.CreateVersion7(),
            Created = created,
            MemoryUsage = memoryUsage,
            CpuUsage = cpuUsage,
            RxBytes = rxBytes,
            TxBytes = txBytes,
            PlatformId = platformId != null ? platformId.Value : Guid.Empty
        };

}