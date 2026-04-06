using Application.Features.Actors.Commands;

namespace WebApi.Routes.Endpoints.Resources.Actors;

public sealed record PatchActorEnabledInput(bool IsEnabled)
{
    internal PatchActorEnabled ToCommand(Guid id) => new(id, IsEnabled);
}
