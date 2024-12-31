using Application.Features.Containers.Models;
using Infrastructure.Entities;
using Riok.Mapperly.Abstractions;

namespace Application;

[Mapper(RequiredMappingStrategy = RequiredMappingStrategy.Source)]
internal static partial class Mapper
{
    public static partial IEnumerable<ContainerPort> Map(this IList<PortRequest> ports);
}