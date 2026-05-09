using Hosting.Common;

namespace Domain.Entities.Identity;

public class ResourceAccess
{
    public Guid Id { get; private set; }
    public Guid ActorId { get; private set; }
    public Guid ResourceId { get; private set; }

    public ResourceType ResourceType { get; private set; }
    public PermissionLevel PermissionLevel { get; private set; }
    public IReadOnlyList<SpecificPermission> SpecificPermissions { get; private set; } = [];

    public static ResourceAccess Create(
        ResourceType resourceType,
        Guid resourceId,
        Guid actorId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions = null)
    {
        var normalizedSpecificPermissions = Permission.NormalizeSpecificPermissions(resourceType, permissionLevel, specificPermissions);

        return new()
        {
            Id = Guid.CreateVersion7(),
            ResourceType = resourceType,
            ResourceId = resourceId,
            ActorId = actorId,
            PermissionLevel = permissionLevel,
            SpecificPermissions = normalizedSpecificPermissions,
        };

    }

    public static ResourceAccess FromPersistence(
        Guid id,
        ResourceType resourceType,
        Guid resourceId,
        Guid actorId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions = null)
    {
        return new()
        {
            Id = id,
            ResourceType = resourceType,
            ResourceId = resourceId,
            ActorId = actorId,
            PermissionLevel = permissionLevel,
            SpecificPermissions = specificPermissions?.ToList() ?? [],
        };
    }
}
