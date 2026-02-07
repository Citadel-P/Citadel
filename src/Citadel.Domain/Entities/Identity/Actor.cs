namespace Domain.Entities.Identity;

public sealed class Actor
{
    public Guid Id { get; private set; }
    public ActorType Type { get; private set; }
    public  string Name { get; private set; } = null!;

    public static Actor FromPersistence(Guid id, ActorType type, string name)
    {
        return new Actor
        {
            Id = id,
            Type = type,
            Name = name
        };
    }
}
