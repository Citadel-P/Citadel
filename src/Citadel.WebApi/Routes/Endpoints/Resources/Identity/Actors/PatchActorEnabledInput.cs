using Application.Features.Identity.Actors.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Actors;

public sealed record PatchActorEnabledInput(bool IsEnabled)
{
    internal PatchActorEnabled ToCommand(Guid id) => new(id, IsEnabled);
}
