namespace Domain.Entities.Identity;

public class RefreshToken
{
    public Guid Id { get; private set; }
    public Guid UserId { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;

    public User User { get; private set; } = null!;

    public static RefreshToken Create(Guid id, Guid userId) => new()
    {
        Id = id,
        UserId = userId,
        CreatedAt = DateTime.UtcNow,
    };
}
