using Contracts.Broker.Models;
using Infrastructure.Entities;
using Riok.Mapperly.Abstractions;
using WebApi.Controllers.V1.Resources.Containers;
using WebApi.Controllers.V1.Resources.Platforms;

namespace WebApi.Controllers.V1.Resources;


[Mapper(RequiredMappingStrategy = RequiredMappingStrategy.Target)]
internal static partial class Mapper
{
    public static partial PlatformView Map(Platform platform);
    public static partial ContainerInfoView Map(ContainerInfo containerInfo);
    public static partial PortView Map(ContainerPort port);
    public static partial ContainerStatView Map(ContainerStat stat);
    public static partial ContainerLogView Map(ContainerLogMessage logs);
}
