namespace Domain.Entities.Identity;

public sealed class ServiceAccount : IAuditedEntity
{
    public Guid Id { get; private set; }
    public string Name { get; private set; }
    public string? Description { get; private set; }
    public Guid ActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    public Guid CreatedByActorId { get; private set; }
    public DateTime UpdatedAt { get; private set; }
    public DateTime? ArchivedAtUtc { get; private set; }

    private ServiceAccount(
        Guid id,
        string name,
        string? description,
        Guid actorId,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt,
        DateTime? archivedAtUtc)
    {
        Id = id;
        Name = name;
        Description = description;
        ActorId = actorId;
        CreatedByActorId = createdByActorId;
        CreatedAt = createdAt;
        UpdatedAt = updatedAt;
        ArchivedAtUtc = archivedAtUtc;
    }

    public static ServiceAccount Create(
        string name,
        string? description,
        Guid actorId,
        Guid createdByActorId,
        DateTime now)
        => new(
            Guid.CreateVersion7(),
            name.Trim(),
            NormalizeDescription(description),
            actorId,
            createdByActorId,
            now,
            now,
            null);

    public void Update(string? name, string? description, DateTime now)
    {
        if (name is not null)
            Name = name.Trim();
        Description = NormalizeDescription(description);
        UpdatedAt = now;
    }

    public void Archive(DateTime now)
    {
        ArchivedAtUtc ??= now;
        UpdatedAt = now;
    }

    public static ServiceAccount FromPersistence(
        Guid id,
        string name,
        string? description,
        Guid actorId,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt,
        DateTime? archivedAtUtc)
        => new(id, name, description, actorId, createdByActorId, createdAt, updatedAt, archivedAtUtc);

    private static string? NormalizeDescription(string? description)
        => string.IsNullOrWhiteSpace(description) ? null : description.Trim();
}

public sealed class ServiceAccountToken
{
    public Guid Id { get; private set; }
    public Guid ServiceAccountId { get; private set; }
    public string Name { get; private set; }
    public byte[] SecretHash { get; private set; }
    public DateTime? ExpiresAtUtc { get; private set; }
    public DateTime? LastUsedAtUtc { get; private set; }
    public DateTime? RevokedAtUtc { get; private set; }
    public Guid? RevokedByActorId { get; private set; }
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAtUtc { get; private set; }

    private ServiceAccountToken(
        Guid id,
        Guid serviceAccountId,
        string name,
        byte[] secretHash,
        DateTime? expiresAtUtc,
        DateTime? lastUsedAtUtc,
        DateTime? revokedAtUtc,
        Guid? revokedByActorId,
        Guid createdByActorId,
        DateTime createdAtUtc)
    {
        Id = id;
        ServiceAccountId = serviceAccountId;
        Name = name;
        SecretHash = secretHash;
        ExpiresAtUtc = expiresAtUtc;
        LastUsedAtUtc = lastUsedAtUtc;
        RevokedAtUtc = revokedAtUtc;
        RevokedByActorId = revokedByActorId;
        CreatedByActorId = createdByActorId;
        CreatedAtUtc = createdAtUtc;
    }

    public static ServiceAccountToken Create(
        Guid id,
        Guid serviceAccountId,
        string name,
        byte[] secretHash,
        DateTime? expiresAtUtc,
        Guid createdByActorId,
        DateTime now)
        => new(id, serviceAccountId, name.Trim(), secretHash, expiresAtUtc, null, null, null, createdByActorId, now);

    public static ServiceAccountToken FromPersistence(
        Guid id,
        Guid serviceAccountId,
        string name,
        byte[] secretHash,
        DateTime? expiresAtUtc,
        DateTime? lastUsedAtUtc,
        DateTime? revokedAtUtc,
        Guid? revokedByActorId,
        Guid createdByActorId,
        DateTime createdAtUtc)
        => new(id, serviceAccountId, name, secretHash, expiresAtUtc, lastUsedAtUtc, revokedAtUtc, revokedByActorId, createdByActorId, createdAtUtc);
}
