using Hosting.Common;

namespace Domain.Entities.Identity;

public class ResourceAccess
{
    public Guid Id { get; private set; }
    public Guid ActorId { get; private set; }
    public Guid ResourceId { get; private set; }

    public ResourceType ResourceType { get; private set; }
    public ResourceAction Action { get; private set; }

    public static ResourceAccess Create(
        ResourceType resourceType,
        Guid resourceId,
        Guid actorId,
        ResourceAction action,
        Guid? id = null)
        => new()
        {
            Id = id ?? Guid.CreateVersion7(),
            ResourceType = resourceType,
            ResourceId = resourceId,
            ActorId = actorId,
            Action = action
        };
}
