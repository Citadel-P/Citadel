using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
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
            status: Enum.Parse<ActivityStatus>(activityEventDto.Status),
            info: activityEventDto.Info != null ? JsonSerializer.Deserialize(activityEventDto.Info, EventInfoJsonContext.Default.ActivityEventInfo) : null,
            createdByActorId: activityEventDto.CreatedByActorId,
            createdAt: activityEventDto.CreatedAt,
            platform: activityEventDto.Platform_Name == null ? null : Platform.FromPersistence(id: activityEventDto.PlatformId.Value, name: activityEventDto.Platform_Name, address: string.Empty,
                networkCount: 0, volumeCount: 0, imageCount: 0, cpuCount: 0, memTotal: 0, status: activityEventDto.Platform_Status != null ? Enum.Parse<PlatformStatus>(activityEventDto.Platform_Status): PlatformStatus.Offline, connectorType: PlatformConnectorType.Unknown, platformDescriptor: null),
            actor: activityEventDto.Actor_Name == null ? null : Actor.FromPersistence(id: activityEventDto.CreatedByActorId , type: activityEventDto.Actor_Type != null ? Enum.Parse<ActorType>(activityEventDto.Actor_Type) : ActorType.User, actorMetadata: activityEventDto.Actor_Name != null ? new ActorMetadata(activityEventDto.Actor_Name) : null, isEnabled: true) );
    }
}
