using Application.Features.Stacks.Queries;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StackStatsView(IEnumerable<StackContainerStatsView> Containers)
{
    internal static StackStatsView Map(IEnumerable<StackContainerStats> stats)
        => new(stats.Select(StackContainerStatsView.Map));
}

public sealed record StackContainerStatsView(
    string ContainerId,
    string ContainerName,
    IEnumerable<ContainerStatView> Stats)
{
    internal static StackContainerStatsView Map(StackContainerStats containerStats)
        => new(
            ContainerId: containerStats.ContainerId,
            ContainerName: containerStats.ContainerName,
            Stats: ContainerStatView.Map(containerStats.Stats));
}
