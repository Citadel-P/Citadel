using Domain.Entities.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public record struct PlatformStatView(
    long Created,
    double TxBytes,
    double RxBytes,
    double CpuUsage,
    double MemoryUsage,
    long? DiskUsedBytes,
    long? DiskTotalBytes,
    double? DiskUsage)
{
    internal static List<PlatformStatView> Map(IEnumerable<PlatformStat> stats)
        => [.. stats.Select(Map)];

    internal static PlatformStatView Map(PlatformStat stat)
        => new(
            Created: stat.Created,
            TxBytes: stat.TxBytes,
            RxBytes: stat.RxBytes,
            CpuUsage: stat.CpuUsage,
            MemoryUsage: stat.MemoryUsage,
            DiskUsedBytes: stat.DiskUsedBytes,
            DiskTotalBytes: stat.DiskTotalBytes,
            DiskUsage: stat.DiskUsage);
}

public sealed record PlatformStatsView(IEnumerable<PlatformStatView> Stats)
{
    internal static PlatformStatsView Map(IEnumerable<PlatformStat> stats)
        => new(PlatformStatView.Map(stats));
}
