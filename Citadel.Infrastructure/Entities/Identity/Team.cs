namespace Infrastructure.Entities.Identity;

public class Team
{
    public Guid Id { get; private set; }
    public Guid RoleId { get; private set; }
    public string Name { get; private set; }
    public Role Role { get; private set; }
    public ICollection<User> Users { get; } = [];

    public static Team Create(string name, Guid roleId) => new()
    {
        Id = Guid.CreateVersion7(),
        Name = name,
        RoleId = roleId
    };
}