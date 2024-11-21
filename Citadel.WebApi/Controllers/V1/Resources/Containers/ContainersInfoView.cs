using Infrastructure.Entities;

namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record ContainersInfoView(IEnumerable<ContainerInfoView> Containers)
{
    internal static ContainersInfoView Map(IEnumerable<ContainerInfo> containersInfo)
        => new(ContainerInfoView.Map(containersInfo));
}
