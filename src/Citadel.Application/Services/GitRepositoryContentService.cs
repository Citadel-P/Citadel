using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal interface IGitRepositoryPathNormalizer
{
    Result<string> NormalizeDirectory(string? path);
    Result<string> NormalizeFile(string path);
}

internal sealed class GitRepositoryPathNormalizer : IGitRepositoryPathNormalizer
{
    private const int MaximumCharacters = 4096;
    private const int MaximumUtf8Bytes = 16 * 1024;

    public Result<string> NormalizeDirectory(string? path)
        => Normalize(path, requireValue: false, trimTrailingSeparator: true);

    public Result<string> NormalizeFile(string path)
        => Normalize(path, requireValue: true, trimTrailingSeparator: false);

    private static Result<string> Normalize(
        string? path,
        bool requireValue,
        bool trimTrailingSeparator)
    {
        if (path is null or "")
        {
            return requireValue
                ? Result.Failure<string>(new BadRequestError("Repository file path is required."))
                : Result.Success(string.Empty);
        }

        if (path.Length > MaximumCharacters || Encoding.UTF8.GetByteCount(path) > MaximumUtf8Bytes)
            return Result.Failure<string>(new BadRequestError("Repository path is too long."));

        if (path.IndexOf('\0') >= 0)
            return Result.Failure<string>(new BadRequestError("Repository path contains an invalid character."));

        if (path.Contains('\\')
            || path.StartsWith("/", StringComparison.Ordinal)
            || IsDrivePath(path))
        {
            return Result.Failure<string>(new BadRequestError("Repository path must be relative and use '/'."));
        }

        var normalized = path;
        while (normalized.StartsWith("./", StringComparison.Ordinal))
            normalized = normalized[2..];

        while (normalized.Contains("//", StringComparison.Ordinal))
            normalized = normalized.Replace("//", "/", StringComparison.Ordinal);

        if (trimTrailingSeparator)
            normalized = normalized.TrimEnd('/');
        else if (normalized.EndsWith("/", StringComparison.Ordinal))
            return Result.Failure<string>(new BadRequestError("Repository file path cannot end with '/'."));

        if (normalized.Length == 0)
        {
            return requireValue
                ? Result.Failure<string>(new BadRequestError("Repository file path is required."))
                : Result.Success(string.Empty);
        }

        var segments = normalized.Split('/');
        if (segments.Any(static segment => segment.Length == 0 || segment is "." or ".."))
            return Result.Failure<string>(new BadRequestError("Repository path contains an invalid segment."));

        if (segments[0].Equals(".git", StringComparison.OrdinalIgnoreCase))
            return Result.Failure<string>(new BadRequestError("Repository metadata cannot be browsed."));

        return Result.Success(normalized);
    }

    private static bool IsDrivePath(string path)
        => path.Length >= 2 && char.IsAsciiLetter(path[0]) && path[1] == ':';
}

internal interface IGitRepositoryContentService
{
    Task<Result<GitRepositoryDirectoryListing>> ListDirectoryAsync(
        Guid repositoryId,
        string? commitSha,
        string? path,
        CancellationToken cancellationToken);

    Task<Result<GitRepositoryFileContent>> ReadFileAsync(
        Guid repositoryId,
        string? commitSha,
        string path,
        CancellationToken cancellationToken);

    Task<Result<GitCommitComparison>> CompareAsync(
        Guid repositoryId,
        string baseCommitSha,
        string headCommitSha,
        CancellationToken cancellationToken);
}

internal sealed class GitRepositoryContentService(
    IUnitOfWork unitOfWork,
    IGitCliRepository gitCliRepository,
    IGitRepositoryPathNormalizer pathNormalizer,
    ILogger<GitRepositoryContentService> logger)
    : IGitRepositoryContentService
{
    private const int MaximumDirectoryEntries = 1000;
    private const int MaximumTextPreviewBytes = 1024 * 1024;
    private const int MaximumComparisonEntries = 1000;
    private const int MaximumStructuredOutputBytes = 8 * 1024 * 1024;
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    public async Task<Result<GitRepositoryDirectoryListing>> ListDirectoryAsync(
        Guid repositoryId,
        string? commitSha,
        string? path,
        CancellationToken cancellationToken)
    {
        var normalizedPathResult = pathNormalizer.NormalizeDirectory(path);
        if (normalizedPathResult.IsFailure(out var pathError, out var normalizedPath))
            return Result.Failure<GitRepositoryDirectoryListing>(pathError);

        var contextResult = await ResolveContextAsync(repositoryId, commitSha, cancellationToken);
        if (contextResult.IsFailure(out var contextError, out var context))
            return Result.Failure<GitRepositoryDirectoryListing>(contextError);

        var treeResult = await gitCliRepository.ListTreeAsync(
            context.CachePath,
            context.CommitSha,
            normalizedPath,
            MaximumDirectoryEntries,
            MaximumStructuredOutputBytes,
            cancellationToken);
        if (treeResult.IsFailure(out var treeError, out var tree))
        {
            return Result.Failure<GitRepositoryDirectoryListing>(
                MapGitReadError(treeError.Message, normalizedPath));
        }

        var entries = tree.Entries
            .Select(static entry => new GitRepositoryEntry(
                entry.Name,
                entry.Path,
                entry.Type,
                entry.Size,
                entry.Mode,
                entry.Type == GitRepositoryEntryType.Submodule ? entry.ObjectId : null))
            .OrderBy(static entry => SortBucket(entry.Type))
            .ThenBy(static entry => entry.Name, StringComparer.OrdinalIgnoreCase)
            .ThenBy(static entry => entry.Name, StringComparer.Ordinal)
            .ToArray();

        return new GitRepositoryDirectoryListing(
            context.Repository.Id,
            context.CommitSha,
            normalizedPath,
            entries,
            tree.IsTruncated,
            GitRepositoryProviderLinks.GetRepositoryUrl(context.Repository));
    }

    public async Task<Result<GitRepositoryFileContent>> ReadFileAsync(
        Guid repositoryId,
        string? commitSha,
        string path,
        CancellationToken cancellationToken)
    {
        var normalizedPathResult = pathNormalizer.NormalizeFile(path);
        if (normalizedPathResult.IsFailure(out var pathError, out var normalizedPath))
            return Result.Failure<GitRepositoryFileContent>(pathError);

        var contextResult = await ResolveContextAsync(repositoryId, commitSha, cancellationToken);
        if (contextResult.IsFailure(out var contextError, out var context))
            return Result.Failure<GitRepositoryFileContent>(contextError);

        var entryResult = await gitCliRepository.GetTreeEntryAsync(
            context.CachePath,
            context.CommitSha,
            normalizedPath,
            cancellationToken);
        if (entryResult.IsFailure(out var entryError, out var entry))
            return Result.Failure<GitRepositoryFileContent>(MapGitReadError(entryError.Message, normalizedPath));

        if (entry is null)
            return Result.Failure<GitRepositoryFileContent>(new NotFoundError("Repository file does not exist."));

        if (entry.Type == GitRepositoryEntryType.Directory)
            return Result.Failure<GitRepositoryFileContent>(new BadRequestError("Repository path is a directory."));

        var providerUrl = GitRepositoryProviderLinks.GetFileUrl(
            context.Repository,
            context.CommitSha,
            normalizedPath);

        if (entry.Type == GitRepositoryEntryType.Submodule)
        {
            return new GitRepositoryFileContent(
                context.Repository.Id,
                context.CommitSha,
                normalizedPath,
                entry.Type,
                0,
                IsBinary: false,
                IsTruncated: false,
                Content: null,
                PreviewUnavailableReason: "Submodule browsing is not supported in this version.",
                providerUrl);
        }

        if (entry.Size is null)
            return Result.Failure<GitRepositoryFileContent>(new BadGatewayError("Repository file size is unavailable."));

        if (entry.Size > MaximumTextPreviewBytes)
        {
            return new GitRepositoryFileContent(
                context.Repository.Id,
                context.CommitSha,
                normalizedPath,
                entry.Type,
                entry.Size.Value,
                IsBinary: false,
                IsTruncated: true,
                Content: null,
                PreviewUnavailableReason: "File is larger than the preview limit.",
                providerUrl);
        }

        var blobResult = await gitCliRepository.ReadBlobAsync(
            context.CachePath,
            entry.ObjectId,
            MaximumTextPreviewBytes,
            cancellationToken);
        if (blobResult.IsFailure(out var blobError, out var blob))
            return Result.Failure<GitRepositoryFileContent>(MapGitReadError(blobError.Message, normalizedPath));

        if (blob.IsTruncated)
        {
            return new GitRepositoryFileContent(
                context.Repository.Id,
                context.CommitSha,
                normalizedPath,
                entry.Type,
                entry.Size.Value,
                IsBinary: false,
                IsTruncated: true,
                Content: null,
                PreviewUnavailableReason: "File is larger than the preview limit.",
                providerUrl);
        }

        if (blob.Content.Span.Contains((byte)0))
        {
            return new GitRepositoryFileContent(
                context.Repository.Id,
                context.CommitSha,
                normalizedPath,
                entry.Type,
                entry.Size.Value,
                IsBinary: true,
                IsTruncated: false,
                Content: null,
                PreviewUnavailableReason: "Binary files cannot be previewed.",
                providerUrl);
        }

        string content;
        try
        {
            content = StrictUtf8.GetString(blob.Content.Span);
        }
        catch (DecoderFallbackException)
        {
            return new GitRepositoryFileContent(
                context.Repository.Id,
                context.CommitSha,
                normalizedPath,
                entry.Type,
                entry.Size.Value,
                IsBinary: true,
                IsTruncated: false,
                Content: null,
                PreviewUnavailableReason: "File is not valid UTF-8 text.",
                providerUrl);
        }

        return new GitRepositoryFileContent(
            context.Repository.Id,
            context.CommitSha,
            normalizedPath,
            entry.Type,
            entry.Size.Value,
            IsBinary: false,
            IsTruncated: false,
            content,
            PreviewUnavailableReason: null,
            providerUrl);
    }

    public async Task<Result<GitCommitComparison>> CompareAsync(
        Guid repositoryId,
        string baseCommitSha,
        string headCommitSha,
        CancellationToken cancellationToken)
    {
        if (!IsFullCommitSha(baseCommitSha) || !IsFullCommitSha(headCommitSha))
            return Result.Failure<GitCommitComparison>(new BadRequestError("Both revisions must be full commit SHAs."));

        var repositoryResult = await LoadRepositoryAsync(repositoryId, cancellationToken);
        if (repositoryResult.IsFailure(out var repositoryError, out var repositoryContext))
            return Result.Failure<GitCommitComparison>(repositoryError);

        var baseResult = await ResolveExplicitCommitAsync(
            repositoryContext.CachePath,
            baseCommitSha,
            cancellationToken);
        if (baseResult.IsFailure(out var baseError, out var resolvedBase))
            return Result.Failure<GitCommitComparison>(baseError);

        var headResult = await ResolveExplicitCommitAsync(
            repositoryContext.CachePath,
            headCommitSha,
            cancellationToken);
        if (headResult.IsFailure(out var headError, out var resolvedHead))
            return Result.Failure<GitCommitComparison>(headError);

        var comparisonResult = await gitCliRepository.CompareCommitsAsync(
            repositoryContext.CachePath,
            resolvedBase,
            resolvedHead,
            MaximumComparisonEntries,
            MaximumStructuredOutputBytes,
            cancellationToken);
        if (comparisonResult.IsFailure(out var comparisonError, out var comparison))
        {
            LogGitFailure(repositoryId, "compare", resolvedHead, comparisonError.Message);
            return Result.Failure<GitCommitComparison>(
                new BadGatewayError("Repository commits could not be compared."));
        }

        return new GitCommitComparison(
            repositoryId,
            resolvedBase,
            resolvedHead,
            comparison.Files,
            comparison.IsTruncated);
    }

    private async Task<Result<RepositoryContentContext>> ResolveContextAsync(
        Guid repositoryId,
        string? requestedCommitSha,
        CancellationToken cancellationToken)
    {
        var repositoryResult = await LoadRepositoryAsync(repositoryId, cancellationToken);
        if (repositoryResult.IsFailure(out var repositoryError, out var repositoryContext))
            return Result.Failure<RepositoryContentContext>(repositoryError);

        if (!string.IsNullOrEmpty(requestedCommitSha))
        {
            var commitResult = await ResolveExplicitCommitAsync(
                repositoryContext.CachePath,
                requestedCommitSha,
                cancellationToken);
            if (commitResult.IsFailure(out var commitError, out var commitSha))
                return Result.Failure<RepositoryContentContext>(commitError);

            return new RepositoryContentContext(
                repositoryContext.Repository,
                repositoryContext.CachePath,
                commitSha);
        }

        var branch = repositoryContext.Repository.DefaultBranch ?? "main";
        var repositoryRef = await unitOfWork.GitRepositories.GetRefAsync(
            repositoryId,
            branch,
            cancellationToken);

        if (repositoryRef is not null)
        {
            if (!IsFullCommitSha(repositoryRef.ResolvedCommitSha))
            {
                return Result.Failure<RepositoryContentContext>(
                    new ConflictError("The synchronized repository revision is unavailable."));
            }

            var storedCommit = await gitCliRepository.ResolveCommitAsync(
                repositoryContext.CachePath,
                repositoryRef.ResolvedCommitSha!,
                cancellationToken);
            if (storedCommit.IsFailure(out var storedError, out var resolvedStoredCommit))
            {
                LogGitFailure(repositoryId, "resolve", repositoryRef.ResolvedCommitSha!, storedError.Message);
                return Result.Failure<RepositoryContentContext>(
                    MapCommitResolutionError(storedError.Message));
            }

            return new RepositoryContentContext(
                repositoryContext.Repository,
                repositoryContext.CachePath,
                resolvedStoredCommit);
        }

        var fallback = await gitCliRepository.ResolveSnapshotCommitAsync(
            repositoryContext.CachePath,
            branch,
            cancellationToken);
        if (fallback.IsFailure(out var fallbackError, out var fallbackCommit))
        {
            LogGitFailure(repositoryId, "resolve-default", string.Empty, fallbackError.Message);
            return Result.Failure<RepositoryContentContext>(
                new ConflictError("The repository has not been synchronized."));
        }

        if (!IsFullCommitSha(fallbackCommit))
        {
            return Result.Failure<RepositoryContentContext>(
                new ConflictError("The synchronized repository revision is unavailable."));
        }

        var resolvedFallback = await ResolveExplicitCommitAsync(
            repositoryContext.CachePath,
            fallbackCommit,
            cancellationToken);
        if (resolvedFallback.IsFailure(out var resolvedFallbackError, out var resolvedCommit))
            return Result.Failure<RepositoryContentContext>(resolvedFallbackError);

        return new RepositoryContentContext(
            repositoryContext.Repository,
            repositoryContext.CachePath,
            resolvedCommit);
    }

    private async Task<Result<RepositoryCacheContext>> LoadRepositoryAsync(
        Guid repositoryId,
        CancellationToken cancellationToken)
    {
        var repository = await unitOfWork.GitRepositories.GetAsync(repositoryId, cancellationToken);
        if (repository is null)
        {
            return Result.Failure<RepositoryCacheContext>(
                new NotFoundError($"Git repository with ID {repositoryId} does not exist."));
        }

        var cachePath = ApplicationStoragePaths.GetRepositoryCachePath(repository);
        if (!Directory.Exists(Path.Combine(cachePath, ".git")))
        {
            return Result.Failure<RepositoryCacheContext>(
                new ConflictError("The repository has not been synchronized."));
        }

        return new RepositoryCacheContext(repository, cachePath);
    }

    private async Task<Result<string>> ResolveExplicitCommitAsync(
        string cachePath,
        string commitSha,
        CancellationToken cancellationToken)
    {
        if (!IsFullCommitSha(commitSha))
            return Result.Failure<string>(new BadRequestError("Revision must be a full commit SHA."));

        var result = await gitCliRepository.ResolveCommitAsync(
            cachePath,
            commitSha.ToLowerInvariant(),
            cancellationToken);
        if (result.IsFailure(out var error, out var resolved))
            return Result.Failure<string>(MapCommitResolutionError(error.Message));

        return resolved;
    }

    private Error MapGitReadError(string error, string path)
    {
        if (IsInvalidRepositoryCache(error))
            return new ConflictError("The synchronized repository cache is invalid.");

        if (error.Contains("does not exist", StringComparison.OrdinalIgnoreCase))
            return new NotFoundError("Repository path does not exist.");

        if (error.Contains("not a directory", StringComparison.OrdinalIgnoreCase))
            return new BadRequestError("Repository path is not a directory.");

        logger.LogWarning(
            "Git repository browser failed for path {Path}: {Error}",
            path,
            TruncateForLog(error));
        return new BadGatewayError("Repository content could not be read.");
    }

    private static Error MapCommitResolutionError(string error)
        => IsInvalidRepositoryCache(error)
            ? new ConflictError("The synchronized repository cache is invalid.")
            : new NotFoundError("Commit not found in the local repository cache.");

    private static bool IsInvalidRepositoryCache(string error)
        => error.Contains("not a git repository", StringComparison.OrdinalIgnoreCase)
            || error.Contains("not a gitdir", StringComparison.OrdinalIgnoreCase)
            || error.Contains("bad gitfile", StringComparison.OrdinalIgnoreCase)
            || error.Contains("invalid gitfile", StringComparison.OrdinalIgnoreCase)
            || error.Contains("cannot change to", StringComparison.OrdinalIgnoreCase);

    private void LogGitFailure(Guid repositoryId, string operation, string commitSha, string error)
    {
        logger.LogWarning(
            "Git repository browser operation {Operation} failed for repository {RepositoryId} at {CommitSha}: {Error}",
            operation,
            repositoryId,
            ShortSha(commitSha),
            TruncateForLog(error));
    }

    private static int SortBucket(GitRepositoryEntryType type)
        => type switch
        {
            GitRepositoryEntryType.Directory => 0,
            GitRepositoryEntryType.File => 1,
            GitRepositoryEntryType.Symlink => 2,
            GitRepositoryEntryType.Submodule => 3,
            _ => 4
        };

    private static bool IsFullCommitSha(string? value)
        => value is not null && value.Length is 40 or 64 && value.All(Uri.IsHexDigit);

    private static string ShortSha(string value)
        => value.Length <= 12 ? value : value[..12];

    private static string TruncateForLog(string value)
        => value.Length <= 4096 ? value : value[..4096];

    private sealed record RepositoryCacheContext(GitRepository Repository, string CachePath);
    private sealed record RepositoryContentContext(GitRepository Repository, string CachePath, string CommitSha);
}

internal static class GitRepositoryProviderLinks
{
    public static string? GetRepositoryUrl(GitRepository repository)
        => TryGetHttpRepositoryUri(repository.Url, out var uri)
            ? uri.AbsoluteUri.TrimEnd('/')
            : null;

    public static string? GetFileUrl(
        GitRepository repository,
        string commitSha,
        string path)
    {
        if (!TryGetHttpRepositoryUri(repository.Url, out var repositoryUri))
            return null;

        var providerPath = GetProviderPath(repository, repositoryUri, commitSha, path);
        return providerPath is null
            ? null
            : new Uri(repositoryUri, providerPath).AbsoluteUri;
    }

    private static string? GetProviderPath(
        GitRepository repository,
        Uri repositoryUri,
        string commitSha,
        string path)
    {
        var host = repositoryUri.Host;
        var encodedPath = string.Join('/', path.Split('/').Select(Uri.EscapeDataString));
        if (host.Equals("github.com", StringComparison.OrdinalIgnoreCase))
            return $"{repositoryUri.AbsolutePath.TrimEnd('/')}/blob/{commitSha}/{encodedPath}";

        if (host.Equals("gitlab.com", StringComparison.OrdinalIgnoreCase)
            || repository.Webhook is { Enabled: true, Provider: WebhookProvider.GitLab })
        {
            return $"{repositoryUri.AbsolutePath.TrimEnd('/')}/-/blob/{commitSha}/{encodedPath}";
        }

        if (host.Equals("bitbucket.org", StringComparison.OrdinalIgnoreCase))
            return $"{repositoryUri.AbsolutePath.TrimEnd('/')}/src/{commitSha}/{encodedPath}";

        return null;
    }

    private static bool TryGetHttpRepositoryUri(string value, out Uri uri)
    {
        uri = null!;
        if (!Uri.TryCreate(value, UriKind.Absolute, out var parsed)
            || parsed.Scheme is not ("http" or "https"))
        {
            return false;
        }

        var builder = new UriBuilder(parsed)
        {
            UserName = string.Empty,
            Password = string.Empty,
            Query = string.Empty,
            Fragment = string.Empty
        };
        var path = builder.Path.TrimEnd('/');
        if (path.EndsWith(".git", StringComparison.OrdinalIgnoreCase))
            path = path[..^4];
        builder.Path = path;
        uri = builder.Uri;
        return true;
    }
}
