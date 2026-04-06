using Hosting.Common;

namespace Domain.Entities.Identity;

public class Permission
{
    public Guid Id { get; private set; }
    public Guid RoleId { get; private set; }
    public ResourceType ResourceType { get; private set; }
    public ResourceAction ResourceAction { get; private set; }

    public static Permission Create(Guid roleId, ResourceType resourceType, ResourceAction resourceAction, Guid? id = null) => new ()
    {
        Id = id ?? Guid.CreateVersion7(),
        RoleId = roleId,
        ResourceType = resourceType,
        ResourceAction = resourceAction,
    };
}