using Infrastructure.Entities;
using WebApi.Controllers.V1.Resources.Platforms;

namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record ContainerInfoView (
    Guid Id,
    string ContainerId,
    string Name,
    string Image,
    DateTimeOffset Created, 
    string State,
    string Status,
    IEnumerable<ContainerStatView> Stats,
    IEnumerable<PortView> Ports,
    IDictionary<string, string> Labels,
    PlatformView Platform)
{
    internal static IEnumerable<ContainerInfoView> Map(IEnumerable<ContainerInfo> containersInfo)
        => containersInfo.Select(Mapper.Map);

    internal static ContainerInfoView Map(ContainerInfo container)
    => new(
        Id: container.Id,
        ContainerId: container.ContainerId,
        Name: container.Name,
        Image: container.Image,
        Created: container.Created,
        State: container.State,
        Status: container.Status,
        Labels: container.Labels,
        Stats: container.Stats.Select(Mapper.Map),
        Ports: container.Ports.Select(Mapper.Map),
        Platform: PlatformView.Map(container.Platform));
};
