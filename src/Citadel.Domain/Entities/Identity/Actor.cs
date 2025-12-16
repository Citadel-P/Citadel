namespace Domain.Entities.Identity;

public sealed class Actor
{
    public Guid Id { get; private set; }
    public ActorType Type { get; private set; }
    public  string Name { get; private set; } = null!;

    public static readonly Guid SystemId =
        Guid.Parse("00000000-0000-0000-0000-000000000001");
}
