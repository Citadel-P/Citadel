using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record struct ContainerStatView(
        Guid ContainerId,
        double MemoryActive,
        double MemoryCache,
        double CpuUsage,
        double MemoryLimit,
        double RxBytes,
        double TxBytes,
        long Created)
{
    internal static List<ContainerStatView> Map(IEnumerable<ContainerStat> stats)
        => [.. stats.Select(Map)];

    internal static ContainerStatView Map(ContainerStat stats)
        => new (
            ContainerId: stats.ContainerId,
            MemoryActive: stats.MemoryActive ?? 0,
            MemoryCache: stats.MemoryCache ?? 0,
            CpuUsage: stats.CpuUsage ?? 0,
            MemoryLimit: stats.MemoryLimit ?? 0,
            RxBytes: stats.RxBytes ?? 0,
            TxBytes: stats.TxBytes ?? 0,
            Created: stats.Created ?? 0);
}

public sealed record ContainerStatsView(IEnumerable<ContainerStatView> Stats)
{
    internal static ContainerStatsView Map(IEnumerable<ContainerStat> stats)
        => new(ContainerStatView.Map(stats));
}
