namespace Domain.Entities.Identity;

public sealed class InstanceSetupState
{
    public const short SingletonId = 1;

    public short Id { get; private set; } = SingletonId;
    public DateTimeOffset? InitializedAt { get; private set; }
    public Guid? InitialAdministratorActorId { get; private set; }
    public DateTimeOffset CreatedAt { get; private set; }
    public DateTimeOffset UpdatedAt { get; private set; }

    public bool RequiresSetup => InitializedAt is null;

    public bool TryComplete(Guid administratorActorId, DateTimeOffset completedAt)
    {
        if (!RequiresSetup)
            return false;

        InitialAdministratorActorId = administratorActorId;
        InitializedAt = completedAt;
        UpdatedAt = completedAt;
        return true;
    }

    public static InstanceSetupState FromPersistence(
        short id,
        DateTimeOffset? initializedAt,
        Guid? initialAdministratorActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt)
    {
        return new InstanceSetupState
        {
            Id = id,
            InitializedAt = initializedAt,
            InitialAdministratorActorId = initialAdministratorActorId,
            CreatedAt = createdAt,
            UpdatedAt = updatedAt
        };
    }
}
