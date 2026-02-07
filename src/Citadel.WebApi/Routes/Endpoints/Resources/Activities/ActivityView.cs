using Domain;
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Activities;

public sealed record ActivityView(
    Guid Id, 
    Guid? PlatformId,
    Guid? ResourceId,
    string PlatformName,
    string ResourceName,
    PlatformStatus PlatformStatus,
    ActivityResourceType ResourceType,
    ActivityEventType EventType,
    ActivityStatus Status,
    DateTime CreatedAt,
    EventInfo Info,
    Guid ActorId,
    string ActorName,
    ActorType ActorType
    )
{
    internal static ActivityView Map(ActivityEvent activity)
    {
        return new(
            activity.Id,
            activity.PlatformId,
            activity.ResourceId,
            activity.Platform?.Name ?? "Unknown",
            activity.ResourceName,
            activity.Platform?.Status ?? PlatformStatus.Offline,
            activity.ResourceType,
            activity.EventType,
            activity.Status,
            activity.CreatedAt,
            activity.Info,
            activity.CreatedByActorId,
            ActorName: activity.Actor?.Name ?? "Unknown",
            ActorType: activity.Actor?.Type ?? ActorType.User
            );
    }
}