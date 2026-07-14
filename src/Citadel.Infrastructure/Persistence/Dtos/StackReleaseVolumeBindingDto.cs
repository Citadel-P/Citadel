namespace Infrastructure.Persistence.Dtos;

internal sealed record StackReleaseVolumeBindingDto(
    Guid Id,
    Guid StackReleaseId,
    Guid PlatformId,
    string VolumeName,
    string? ComposeVolumeName,
    bool IsExternal,
    bool IsAnonymous,
    DateTime CreatedAt)
{
    public StackReleaseVolumeBindingDto()
        : this(Guid.Empty, Guid.Empty, Guid.Empty, string.Empty, null, false, false, DateTime.MinValue)
    {
    }
}
