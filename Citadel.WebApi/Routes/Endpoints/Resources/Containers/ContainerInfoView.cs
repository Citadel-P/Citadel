using Infrastructure;
using Infrastructure.Entities;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerInfoView(
    Guid Id,
    string ContainerId,
    string Name,
    string Image,
    DateTimeOffset Created,
     ContainerStateStatus State,
    string Status,
    string Stack,
    ContainerStatView LastStats,
    IEnumerable<PortView> Ports = null,
    PlatformView Platform = null)
{
    internal static IEnumerable<ContainerInfoView> Map(IEnumerable<ContainerInfo> containersInfo)
        => containersInfo?.Select(Map);

    internal static ContainerInfoView Map(ContainerInfo container)
    => new(
        Id: container.Id,
        ContainerId: container.ContainerId,
        Name: container.Name,
        Image: container.Image,
        Created: DateTimeOffset.FromUnixTimeSeconds(container.Created),
        State: container.State,
        Status: container.Status,
        Stack: container.Stack,
        LastStats: ContainerStatView.Map(container.Stats?.FirstOrDefault()),
        Ports: container.Ports == null ? null : PortView.Map(container.Ports),
        Platform: container.Platform == null ? null : PlatformView.Map(container.Platform));
};