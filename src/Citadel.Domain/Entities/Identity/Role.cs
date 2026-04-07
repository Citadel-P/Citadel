namespace Domain.Entities.Identity;

public class Role
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = string.Empty;
    public IEnumerable<Permission> Permissions { get; private set; } = [];

    public static Role Create(string name, IEnumerable<Permission>? permissions = null)
    {
        return new Role
        {
            Id = Guid.CreateVersion7(),
            Name = name,
            Permissions = permissions ?? []
        };
    }

    public static Role FromPersistence(Guid id, string name, IEnumerable<Permission>? permissions = null)
    {
        return new Role
        {
            Id = id,
            Name = name,
            Permissions = permissions ?? []
        };
    }

    public void Rename(string name) => Name = name;

    public void SetPermissions(IEnumerable<Permission> permissions) => Permissions = permissions;
}
