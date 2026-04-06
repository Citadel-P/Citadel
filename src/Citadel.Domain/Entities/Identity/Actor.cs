namespace Domain.Entities.Identity;

public sealed class Actor
{
    public Guid Id { get; private set; }
    public ActorType Type { get; private set; }
    public ActorMetadata ActorMetadata { get; private set; }
    public static Actor FromPersistence(Guid id, ActorType type, ActorMetadata actorMetadata)
    {
        return new Actor
        {
            Id = id,
            Type = type,
            ActorMetadata = actorMetadata
        };
    }
}
public sealed record ActorMetadata(string Name);