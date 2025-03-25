using Application.Features.Containers.Commands;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record DeleteContainersRequest(string[] ContainersIds, bool? V = false, bool? Force = false, bool? Link = false)
{
    internal DeleteContainers ToCommand()
        => new(ContainersIds, V, Force, Link);
}
