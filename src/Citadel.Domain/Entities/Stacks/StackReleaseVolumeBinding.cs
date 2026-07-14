using Domain.Entities.Backups;

namespace Domain.Entities.Stacks;

public sealed class StackReleaseVolumeBinding(
    Guid stackReleaseId,
    Guid platformId,
    string volumeName,
    string? composeVolumeName = null,
    bool isExternal = false,
    bool isAnonymous = false,
    DateTimeOffset? createdAt = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid StackReleaseId { get; private set; } = stackReleaseId;
    public Guid PlatformId { get; private set; } = platformId;
    public string VolumeName { get; private set; } = BackupRepository.NormalizeName(volumeName);
    public string? ComposeVolumeName { get; private set; } = BackupRepository.NormalizeOptional(composeVolumeName);
    public bool IsExternal { get; private set; } = isExternal;
    public bool IsAnonymous { get; private set; } = isAnonymous;
    public DateTimeOffset CreatedAt { get; private set; } = (createdAt ?? DateTimeOffset.UtcNow).ToUniversalTime();

    public static StackReleaseVolumeBinding FromPersistence(
        Guid id,
        Guid stackReleaseId,
        Guid platformId,
        string volumeName,
        string? composeVolumeName,
        bool isExternal,
        bool isAnonymous,
        DateTimeOffset createdAt)
        => new(stackReleaseId, platformId, volumeName, composeVolumeName, isExternal, isAnonymous, createdAt)
        {
            Id = id
        };
}
