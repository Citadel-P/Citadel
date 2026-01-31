using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class ActivityEventMappers
{
    internal static ActivityEvent ToDomain(this ActivityEventDto activityEventDto)
    {
        return ActivityEvent.FromPersistence(
            id: activityEventDto.Id,
            platformId: activityEventDto.PlatformId,
            resourceId: activityEventDto.ResourceId,
            resourceName: activityEventDto.ResourceName,
            resourceType: Enum.Parse<ActivityResourceType>(activityEventDto.ResourceType),
            eventType: Enum.Parse<ActivityEventType>(activityEventDto.EventType),
            info: JsonSerializer.Deserialize(activityEventDto.Info, EventInfoJsonContext.Default.EventInfo),
            createdByActorId: activityEventDto.CreatedByActorId,
            createdAt: activityEventDto.CreatedAt);
    }
}
