namespace Domain.Entities.Identity;

public class Team
{
    public Guid Id { get; private set; }
    public string Name { get; private set; } = null!;
    public Guid ActorId { get; private set; }
    public ICollection<User> Users { get; } = [];

    public static Team Create(string name, Guid actorId, Guid? id = null) => new()
    {
        Id = id ?? Guid.CreateVersion7(),
        Name = name,
        ActorId = actorId
    };

    public static Team FromPersistence(Guid id, string name, Guid actorId)
        => new()
        {
            Id = id,
            Name = name,
            ActorId = actorId
        };

    public void Rename(string name) => Name = name;
}