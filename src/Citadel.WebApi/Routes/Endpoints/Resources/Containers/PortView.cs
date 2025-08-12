using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public record struct PortView(
    string Ip,
    int PrivatePort,
    int PublicPort)
{
    internal static List<PortView> Map(IEnumerable<ContainerPort> ports)
       => [.. ports.Select(Map)];

    internal static PortView Map(ContainerPort port)
        => new(
            Ip: port.IP,
            PrivatePort: port.PrivatePort ?? 0,
            PublicPort: port.PublicPort ?? 0);
}