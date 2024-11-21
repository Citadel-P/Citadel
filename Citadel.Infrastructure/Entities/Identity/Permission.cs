namespace Infrastructure.Entities.Identity;

public class Permission
{
    /// <summary>
    /// Permission Id
    /// </summary>
    public Guid Id { get; private set; }

    /// <summary>
    /// Role Id
    /// </summary>
    public Guid RoleId { get; private set; }

    /// <summary>
    /// The permission name
    /// </summary>
    public AppPermission PermissionCode { get; private set; }

    public static Permission Create(Guid roleId, AppPermission permission) => new ()
    {
        Id = Guid.CreateVersion7(),
        RoleId = roleId,
        PermissionCode = permission
    };
}