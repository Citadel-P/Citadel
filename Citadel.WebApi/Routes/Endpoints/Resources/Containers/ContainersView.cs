using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainersView(IEnumerable<ContainerView> Containers)
{
    internal static ContainersView Map(IEnumerable<Container> containersInfo)
        => new(ContainerView.Map(containersInfo));
}
