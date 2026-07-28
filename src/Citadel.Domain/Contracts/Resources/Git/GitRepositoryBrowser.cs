namespace Domain.Contracts.Resources.Git;

public enum GitRepositoryEntryType
{
    Directory,
    File,
    Symlink,
    Submodule
}

public sealed record GitRepositoryEntry(
    string Name,
    string Path,
    GitRepositoryEntryType Type,
    long? Size,
    string Mode,
    string? TargetCommitSha = null);

public sealed record GitRepositoryDirectoryListing(
    Guid RepositoryId,
    string CommitSha,
    string Path,
    IReadOnlyList<GitRepositoryEntry> Entries,
    bool IsTruncated,
    string? ProviderRepositoryUrl);

public sealed record GitRepositoryFileContent(
    Guid RepositoryId,
    string CommitSha,
    string Path,
    GitRepositoryEntryType Type,
    long Size,
    bool IsBinary,
    bool IsTruncated,
    string? Content,
    string? PreviewUnavailableReason,
    string? ProviderUrl);

public enum GitChangedPathStatus
{
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged
}

public sealed record GitChangedPath(
    GitChangedPathStatus Status,
    string Path,
    string? PreviousPath);

public sealed record GitCommitComparison(
    Guid RepositoryId,
    string BaseCommitSha,
    string HeadCommitSha,
    IReadOnlyList<GitChangedPath> Files,
    bool IsTruncated);

public sealed record GitTreeEntry(
    string Name,
    string Path,
    GitRepositoryEntryType Type,
    long? Size,
    string Mode,
    string ObjectId);

public sealed record GitTreeListing(
    IReadOnlyList<GitTreeEntry> Entries,
    bool IsTruncated);

public sealed record GitBlob(ReadOnlyMemory<byte> Content, bool IsTruncated);

public sealed record GitChangedPathListing(
    IReadOnlyList<GitChangedPath> Files,
    bool IsTruncated);
