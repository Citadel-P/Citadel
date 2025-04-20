using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record struct ContainerStatView(
        double MemoryUsage,
        double CpuUsage,
        double MemoryLimit,
        long RxBytes,
        long TxBytes,
        long Created)
{
    internal static IEnumerable<ContainerStatView> Map(IEnumerable<ContainerStat> stats)
        => stats.Select(Map);

    internal static ContainerStatView Map(ContainerStat stats)
        => new (
            MemoryUsage: stats?.MemoryUsage ?? 0,
            CpuUsage: stats?.CpuUsage ?? 0,
            MemoryLimit: stats?.MemoryLimit ?? 0,
            RxBytes: stats?.RxBytes ?? 0,
            TxBytes: stats?.TxBytes ?? 0,
            Created: stats?.Created ?? 0);
}

public sealed record ContainerStatsView(IEnumerable<ContainerStatView> Stats)
{
    internal static ContainerStatsView Map(IEnumerable<ContainerStat> stats)
        => new(ContainerStatView.Map(stats));
}