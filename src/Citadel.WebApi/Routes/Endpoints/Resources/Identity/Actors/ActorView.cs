using Domain;
using Domain.Entities.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Actors;

public sealed record ActorView(
    Guid Id,
    string Name,
    ActorType Type,
    bool IsEnabled)
{
    internal static ActorView Map(Actor actor)
        => new(
            actor.Id,
            actor.ActorMetadata.Name,
            actor.Type,
            actor.IsEnabled);
}
