using Domain;

namespace Domain.Contracts.Resources.Volumes;

public sealed record NormalizedVolumePath(
    string ApiPath,
    IReadOnlyList<string> Segments);

public sealed record ListVolumeDirectoryCommand(
    string PlatformAddress,
    Guid PlatformId,
    PlatformConnectorType ConnectorType,
    string VolumeName,
    NormalizedVolumePath Path);

public sealed record DownloadVolumePathCommand(
    string PlatformAddress,
    Guid PlatformId,
    PlatformConnectorType ConnectorType,
    string VolumeName,
    NormalizedVolumePath Path);

public sealed record VolumeDirectoryListing(
    Guid PlatformId,
    string VolumeName,
    string Path,
    IReadOnlyList<VolumeFileEntry> Entries,
    bool IsTruncated);

public sealed record VolumeFileEntry(
    string Name,
    string Path,
    VolumeFileEntryType Type,
    long? Size,
    DateTimeOffset? ModifiedAt,
    string? LinkTarget);

public enum VolumeFileEntryType
{
    Directory,
    File,
    Symlink,
    Other
}

public sealed class VolumeDownloadStream : IAsyncDisposable
{
    public required Guid PlatformId { get; init; }

    public required string VolumeName { get; init; }

    public required string Path { get; init; }

    public required VolumeFileEntryType EntryType { get; init; }

    public required string FileName { get; init; }

    public required string ContentType { get; init; }

    public long? ContentLength { get; init; }

    public required IAsyncEnumerable<ReadOnlyMemory<byte>> Chunks { get; init; }

    public required Func<ValueTask> CleanupAsync { get; init; }

    public ValueTask DisposeAsync()
        => CleanupAsync();
}
