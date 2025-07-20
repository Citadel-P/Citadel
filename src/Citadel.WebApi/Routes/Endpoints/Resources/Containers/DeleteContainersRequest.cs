using Application.Features.Containers.Commands;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record DeleteContainersRequest(string[] ContainerIds, bool? V = false, bool? Force = false, bool? Link = false)
{
    internal DeleteContainers ToCommand()
        => new(ContainerIds, V, Force, Link);
}
