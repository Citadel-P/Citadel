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
        IEnumerable<SpecificPermission>? specificPermissions = null,
        Guid? id = null)
    {
        var normalizedSpecificPermissions = Permission.NormalizeSpecificPermissions(resourceType, permissionLevel, specificPermissions);

        return new()
        {
            Id = id ?? Guid.CreateVersion7(),
            ResourceType = resourceType,
            ResourceId = resourceId,
            ActorId = actorId,
            PermissionLevel = permissionLevel,
            SpecificPermissions = normalizedSpecificPermissions,
        };

    }
}
