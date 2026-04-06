namespace Domain.Entities.Identity;

public class Role
{
    public Guid Id { get; private set; }
    public string Name { get; private set; }

    public static Role Create(string name)
    {
        return new Role
        {
            Id = Guid.CreateVersion7(),
            Name = name
        };
    }
}
