using Hosting.Common.ErrorTypes;
using LightResults;

namespace Domain.Entities.Identity;

public class Role
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = string.Empty;
    public RoleType RoleType { get; private set; }
    public IEnumerable<Permission> Permissions { get; private set; } = [];

    public static Role Create(string name, RoleType roleType, IEnumerable<Permission>? permissions = null)
    {
        return new Role
        {
            Id = Guid.CreateVersion7(),
            Name = name,
            RoleType = roleType,
            Permissions = permissions ?? []
        };
    }

    public static Role FromPersistence(Guid id, string name, RoleType roleType, IEnumerable<Permission>? permissions = null)
    {
        return new Role
        {
            Id = id,
            Name = name,
            RoleType = roleType,
            Permissions = permissions ?? []
        };
    }

    public Result Rename(string name)
    {
        if (RoleType == RoleType.System)
            return Result.Failure(new ConflictError("System roles cannot be updated."));

        Name = name;
        return Result.Success();
    }

    public Result SetPermissions(IEnumerable<Permission> permissions)
    {
        if (RoleType == RoleType.System)
            return Result.Failure(new ConflictError("System roles cannot be updated."));

        Permissions = permissions;
        return Result.Success();
    }
}
