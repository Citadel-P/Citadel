using Infrastructure.Entities;

namespace WebApi.Controllers.V1.Resources.Containers;

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
        DateTimeOffset? Created)
{
    internal static IEnumerable<ContainerStatView> Map(IEnumerable<ContainerStat> stats) 
        => stats.Select(Mapper.Map);
}