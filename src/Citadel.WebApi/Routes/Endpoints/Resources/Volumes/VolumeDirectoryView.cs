using Domain.Contracts.Resources.Volumes;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumeDirectoryView(
    Guid PlatformId,
    string VolumeName,
    string Path,
    IReadOnlyList<VolumeFileEntryView> Entries,
    bool IsTruncated)
{
    internal static VolumeDirectoryView Map(VolumeDirectoryListing listing)
        => new(
            listing.PlatformId,
            listing.VolumeName,
            listing.Path,
            listing.Entries.Select(VolumeFileEntryView.Map).ToArray(),
            listing.IsTruncated);
}

public sealed record VolumeFileEntryView(
    string Name,
    string Path,
    VolumeFileEntryType Type,
    long? Size,
    DateTimeOffset? ModifiedAt,
    string? LinkTarget = null)
{
    internal static VolumeFileEntryView Map(VolumeFileEntry entry)
        => new(entry.Name, entry.Path, entry.Type, entry.Size, entry.ModifiedAt, entry.LinkTarget);
}
