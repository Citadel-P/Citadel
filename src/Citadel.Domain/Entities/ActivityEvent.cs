using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public sealed class ActivityEvent : IAuditedEntity
{
    private static readonly ActivityResourceType[] _resourceTypes =
        Enum.GetValues<ActivityResourceType>();

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; }
    public Guid ResourceId { get; private set; }
    public string ResourceName { get; private set; }
    public ActivityResourceType ResourceType { get; }
    public ActivityEventType EventType { get; private set; }
    public EventInfo Info { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public ActivityEvent(
        Guid platformId,
        Guid resourceId,
        Guid actorId,
        string resourceName,
        ActivityEventType eventType,
        EventInfo info)
    {
        if (platformId == Guid.Empty)
            throw new ArgumentException("PlatformId is required", nameof(platformId));

        if (resourceId == Guid.Empty)
            throw new ArgumentException("ResourceId is required", nameof(resourceId));

        if (actorId == Guid.Empty)
            throw new ArgumentException("ActorId is required", nameof(actorId));

        if (string.IsNullOrWhiteSpace(resourceName))
            throw new ArgumentException("ResourceName is required", nameof(resourceName));

        if (!IsValidInfoForEvent(eventType, info))
            throw new ArgumentException(
                $"EventInfo type '{info.GetType().Name}' does not match EventType '{eventType}'");

        PlatformId = platformId;
        ResourceId = resourceId;
        CreatedByActorId = actorId;
        ResourceName = resourceName;
        EventType = eventType;
        Info = info;
        ResourceType = GetResourceType(eventType);
        CreatedAt = DateTime.UtcNow;
    }

    public static ActivityResourceType GetResourceType(ActivityEventType eventType)
    {
        var name = eventType.ToString();

        foreach (var resource in _resourceTypes)
        {
            if (name.StartsWith(resource.ToString(), StringComparison.Ordinal))
                return resource;
        }

        throw new InvalidOperationException(
            $"EventType '{eventType}' does not map to a ResourceType.");
    }

    public static ActivityEvent FromPersistence(
        Guid id,
        Guid platformId,
        Guid resourceId,
        string resourceName,
        ActivityResourceType resourceType,
        ActivityEventType eventType,
        EventInfo info,
        Guid createdByActorId,
        DateTime createdAt)
    {
        if (!IsValidInfoForEvent(eventType, info))
            throw new ArgumentException(
                $"EventInfo type '{info.GetType().Name}' does not match EventType '{eventType}'");
        if (resourceType != GetResourceType(eventType))
            throw new ArgumentException(
                $"EventType {eventType} does not derive from ResourceType {resourceType}");
        return new ActivityEvent(
            platformId: platformId,
            resourceId: resourceId,
            actorId: createdByActorId,
            resourceName: resourceName,
            eventType: eventType,
            info: info)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }

    private static bool IsValidInfoForEvent(ActivityEventType type, EventInfo info)
    {
        return (type, info) switch
        {
            (ActivityEventType.DeploymentCreated, DeploymentCreated) => true,
            (ActivityEventType.DeploymentUpdated, DeploymentUpdated) => true,
            (ActivityEventType.DeploymentDeleted, DeploymentDeleted) => true,

            // Todo: Add mappings
            _ => false
        };
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DeploymentCreated), nameof(ActivityEventType.DeploymentCreated))]
[JsonDerivedType(typeof(DeploymentUpdated), nameof(ActivityEventType.DeploymentUpdated))]
[JsonDerivedType(typeof(DeploymentDeleted), nameof(ActivityEventType.DeploymentDeleted))]
public abstract record EventInfo;
public sealed record DeploymentCreated(DeploymentSpec Spec) : EventInfo;
public sealed record DeploymentUpdated(DeploymentSpec OldSpec, DeploymentSpec NewSpec) : EventInfo;
public sealed record DeploymentDeleted(string Name) : EventInfo;

