using Domain.Contracts.Resources.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryEntryView(
    string Name,
    string Path,
    GitRepositoryEntryType Type,
    long? Size,
    string Mode,
    string? TargetCommitSha)
{
    internal static GitRepositoryEntryView Map(GitRepositoryEntry entry)
        => new(
            entry.Name,
            entry.Path,
            entry.Type,
            entry.Size,
            entry.Mode,
            entry.TargetCommitSha);
}

public sealed record GitRepositoryDirectoryListingView(
    Guid RepositoryId,
    string CommitSha,
    string Path,
    IReadOnlyList<GitRepositoryEntryView> Entries,
    bool IsTruncated,
    string? ProviderRepositoryUrl)
{
    internal static GitRepositoryDirectoryListingView Map(GitRepositoryDirectoryListing listing)
        => new(
            listing.RepositoryId,
            listing.CommitSha,
            listing.Path,
            [.. listing.Entries.Select(GitRepositoryEntryView.Map)],
            listing.IsTruncated,
            listing.ProviderRepositoryUrl);
}

public sealed record GitRepositoryFileContentView(
    Guid RepositoryId,
    string CommitSha,
    string Path,
    GitRepositoryEntryType Type,
    long Size,
    bool IsBinary,
    bool IsTruncated,
    string? Content,
    string? PreviewUnavailableReason,
    string? ProviderUrl)
{
    internal static GitRepositoryFileContentView Map(GitRepositoryFileContent file)
        => new(
            file.RepositoryId,
            file.CommitSha,
            file.Path,
            file.Type,
            file.Size,
            file.IsBinary,
            file.IsTruncated,
            file.Content,
            file.PreviewUnavailableReason,
            file.ProviderUrl);
}

public sealed record GitChangedPathView(
    GitChangedPathStatus Status,
    string Path,
    string? PreviousPath)
{
    internal static GitChangedPathView Map(GitChangedPath path)
        => new(path.Status, path.Path, path.PreviousPath);
}

public sealed record GitCommitComparisonView(
    Guid RepositoryId,
    string BaseCommitSha,
    string HeadCommitSha,
    IReadOnlyList<GitChangedPathView> Files,
    bool IsTruncated)
{
    internal static GitCommitComparisonView Map(GitCommitComparison comparison)
        => new(
            comparison.RepositoryId,
            comparison.BaseCommitSha,
            comparison.HeadCommitSha,
            [.. comparison.Files.Select(GitChangedPathView.Map)],
            comparison.IsTruncated);
}
