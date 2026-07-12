namespace Domain.Entities.Identity;

public class RefreshToken
{
    public Guid Id { get; private set; }
    public Guid UserId { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime LastSeenAt { get; private set; } = DateTime.UtcNow;
    public DateTime ExpiresAt { get; private set; } = DateTime.UtcNow;
    public string? UserAgent { get; private set; }
    public string? IpAddress { get; private set; }

    public User User { get; private set; } = null!;

    public static RefreshToken Create(
        Guid id,
        Guid userId,
        DateTime expiresAt,
        string? userAgent,
        string? ipAddress)
    {
        var now = DateTime.UtcNow;
        return new RefreshToken
        {
            Id = id,
            UserId = userId,
            CreatedAt = now,
            LastSeenAt = now,
            ExpiresAt = expiresAt,
            UserAgent = userAgent,
            IpAddress = ipAddress,
        };
    }

    public void Touch(DateTime lastSeenAt, string? userAgent, string? ipAddress)
    {
        LastSeenAt = lastSeenAt;
        UserAgent = userAgent;
        IpAddress = ipAddress;
    }
}
