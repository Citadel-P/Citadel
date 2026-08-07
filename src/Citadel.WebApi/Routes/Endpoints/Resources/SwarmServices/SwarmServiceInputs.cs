using Application.Features.SwarmServices.Commands;
using Domain.Entities.SwarmServices;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints.Resources.SwarmServices;

public sealed record CreateSwarmServiceInput(
    string Name,
    Guid PlatformId,
    string? Description,
    SwarmServiceSpec Spec,
    IReadOnlyCollection<Guid>? TagIds = null,
    DuplicateSourceInput? DuplicateSource = null)
{
    internal CreateSwarmService ToCommand() => new(
        Name,
        PlatformId,
        Description,
        Spec,
        TagIds,
        DuplicateSource?.ToActivitySourceResource());
}

public sealed record UpdateSwarmServiceInput(SwarmServiceSpec Spec, long RowVersion)
{
    internal UpdateSwarmService ToCommand(Guid id) => new(id, Spec, RowVersion);
}

public sealed record ScaleSwarmServiceInput(int Replicas);
