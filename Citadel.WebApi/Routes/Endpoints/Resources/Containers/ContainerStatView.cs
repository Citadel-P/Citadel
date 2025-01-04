using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerStatsView(IEnumerable<ContainerStatView> stats)
{
    internal static ContainerStatsView Map(IEnumerable<ContainerStat> stats)
        => new(ContainerStatView.Map(stats));
}
public sealed record ContainerStatView(
        double? MemoryUsage,
        double? CpuUsage,
        double? MemoryLimit,
        ulong? RxBytes,
        ulong? TxBytes,
        string Created)
{
    internal static IEnumerable<ContainerStatView> Map(IEnumerable<ContainerStat> stats)
        => stats.Select(Mapper.Map);
}