using Domain;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class ActorMappers
{
    internal static Actor ToDomain(this ActorDto activityEventDto)
    {
        return Actor.FromPersistence(
            id: activityEventDto.Id,
            type: Enum.Parse<ActorType>(activityEventDto.Type),
            name: activityEventDto.Name
            );
    }
}
