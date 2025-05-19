namespace Infrastructure.Entities.Identity;

public class Team
{
    public Guid Id { get; private set; }
    public Guid RoleId { get; private set; }
    public string Name { get; private set; } = null!;
    public Role Role { get; private set; } = null!;
    public ICollection<User> Users { get; } = [];

    public static Team Create(string name, Guid roleId, Guid? id = null) => new()
    {
        Id = id ?? Guid.CreateVersion7(),
        Name = name,
        RoleId = roleId
    };
}