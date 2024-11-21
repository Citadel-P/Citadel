using Contracts.Broker.Models;
using Infrastructure.Entities;
using Riok.Mapperly.Abstractions;

namespace Application;

[Mapper]
internal static partial class Mapper
{
    public static partial SystemInfo Map(this Infrastructure.SystemInfo systemInfo);
    public static partial IEnumerable<ContainerPort> Map(this IList<PortMessage> ports);
}