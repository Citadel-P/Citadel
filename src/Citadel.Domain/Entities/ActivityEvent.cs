using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public sealed class ActivityEvent : IAuditedEntity
{
    private static readonly ActivityResourceType[] _resourceTypes =
        Enum.GetValues<ActivityResourceType>();

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid? PlatformId { get; private set; }
    public Guid? ResourceId { get; private set; }
    public string ResourceName { get; private set; }
    public ActivityResourceType ResourceType { get; }
    public ActivityStatus Status { get; private set; }
    public ActivityEventType EventType { get; private set; }
    public EventInfo Info { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public Platform? Platform { get; private set; }
    public Actor? Actor { get; private set; }

    public ActivityEvent(
        Guid? platformId,
        Guid? resourceId,
        Guid actorId,
        string resourceName,
        ActivityEventType eventType,
        ActivityStatus status,
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

        if (info != null && !IsValidInfoForEvent(eventType, info))
            throw new ArgumentException(
                $"EventInfo type '{info.GetType().Name}' does not match EventType '{eventType}'");

        PlatformId = platformId;
        ResourceId = resourceId;
        CreatedByActorId = actorId;
        ResourceName = resourceName;
        EventType = eventType;
        Info = info;
        Status = status;
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
        Guid? platformId,
        Guid? resourceId,
        string resourceName,
        ActivityResourceType resourceType,
        ActivityEventType eventType,
        ActivityStatus status,
        EventInfo info,
        Guid createdByActorId,
        DateTime createdAt,
        Platform? platform = null,
        Actor? actor = null)
    {
        if (info != null && !IsValidInfoForEvent(eventType, info))
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
            status: status,
            info: info)
        {
            Id = id,
            CreatedAt = createdAt,
            Platform = platform,
            Actor = actor
        };
    }

    public async Task<ActivityEvent> AssignActor(IUnitOfWork uow, CancellationToken ct)
    {
        var actor = await uow.Actors.GetById(CreatedByActorId, ct);
        Actor = actor;
        return this;
    }

    private static bool IsValidInfoForEvent(ActivityEventType type, EventInfo info)
    {
        return (type, info) switch
        {
            (ActivityEventType.DeploymentCreated, DeploymentCreated) => true,
            (ActivityEventType.DeploymentUpdated, DeploymentUpdated) => true,
            (ActivityEventType.DeploymentDeleted, DeploymentDeleted) => true,
            (ActivityEventType.DeploymentRenamed, DeploymentRenamed) => true,
            (ActivityEventType.DeploymentStarted, DeploymentStarted) => true,
            (ActivityEventType.DeploymentStopped, DeploymentStopped) => true,
            (ActivityEventType.DeploymentApplied, DeploymentApplied) => true,
            (ActivityEventType.DeploymentPaused, DeploymentPaused) => true,
            (ActivityEventType.DeploymentDegraded, DeploymentDegraded) => true,


            // Todo: Add mappings
            _ => false
        };
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DeploymentCreated), nameof(ActivityEventType.DeploymentCreated))]
[JsonDerivedType(typeof(DeploymentUpdated), nameof(ActivityEventType.DeploymentUpdated))]
[JsonDerivedType(typeof(DeploymentRenamed), nameof(ActivityEventType.DeploymentRenamed))]
[JsonDerivedType(typeof(DeploymentDeleted), nameof(ActivityEventType.DeploymentDeleted))]
[JsonDerivedType(typeof(DeploymentStarted), nameof(ActivityEventType.DeploymentStarted))]
[JsonDerivedType(typeof(DeploymentStopped), nameof(ActivityEventType.DeploymentStopped))]
[JsonDerivedType(typeof(DeploymentPaused), nameof(ActivityEventType.DeploymentPaused))]
[JsonDerivedType(typeof(DeploymentApplied), nameof(ActivityEventType.DeploymentApplied))]
[JsonDerivedType(typeof(DeploymentDegraded), nameof(ActivityEventType.DeploymentDegraded))]
public abstract record EventInfo;
public sealed record DeploymentCreated(DeploymentSpec Spec) : EventInfo;
public sealed record DeploymentUpdated(DeploymentSpec OldSpec, DeploymentSpec NewSpec) : EventInfo;
public sealed record DeploymentRenamed(string OldName, string NewName) : EventInfo;
public sealed record DeploymentDeleted(DeploymentSpec Spec) : EventInfo;
public sealed record DeploymentStarted(IEnumerable<string> ContainerIds) : EventInfo;
public sealed record DeploymentStopped(IEnumerable<string> ContainerIds) : EventInfo;
public sealed record DeploymentPaused(IEnumerable<string> ContainerIds) : EventInfo;
public sealed record DeploymentDegraded(string Reason) : EventInfo;
public sealed record DeploymentApplied(DeploymentSpec? Spec, IEnumerable<string>? ContainerIds, string? Reason) : EventInfo;

