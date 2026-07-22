namespace Domain.Entities.Backups;

public sealed class BackupRepositoryValidation(
    Guid backupRepositoryId,
    BackupExecutionLocation location,
    Guid? platformId,
    BackupRepositoryValidationStatus status,
    DateTimeOffset lastValidatedAt,
    string? lastErrorCode,
    string? lastErrorMessage)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public BackupExecutionLocation Location { get; private set; } = location;
    public Guid? PlatformId { get; private set; } = platformId;
    public BackupRepositoryValidationStatus Status { get; private set; } = status;
    public DateTimeOffset LastValidatedAt { get; private set; } = lastValidatedAt.ToUniversalTime();
    public string? LastErrorCode { get; private set; } = BackupRepository.NormalizeOptional(lastErrorCode);
    public string? LastErrorMessage { get; private set; } = BackupRepository.NormalizeOptional(lastErrorMessage);

    public void Validate()
    {
        if (BackupRepositoryId == Guid.Empty)
            throw new ArgumentException("Backup repository ID is required.", nameof(BackupRepositoryId));

        new BackupExecutionContext(Location, PlatformId).Validate();
    }

    public static BackupRepositoryValidation FromPersistence(
        Guid id,
        Guid backupRepositoryId,
        BackupExecutionLocation location,
        Guid? platformId,
        BackupRepositoryValidationStatus status,
        DateTimeOffset lastValidatedAt,
        string? lastErrorCode,
        string? lastErrorMessage)
        => new(backupRepositoryId, location, platformId, status, lastValidatedAt, lastErrorCode, lastErrorMessage)
        {
            Id = id
        };
}
