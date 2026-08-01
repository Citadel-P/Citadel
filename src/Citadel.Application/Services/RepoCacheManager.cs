using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using LightResults;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;

namespace Application.Services;

/// <summary>
/// Manages the lifecycle and synchronization of locally cached Git repositories.
/// </summary>
internal interface IRepoCacheManager
{
    /// <summary>
    /// Ensures the local bare repository is cloned and updated.
    /// </summary>
    /// <returns>The resolved commit hash (SHA)</returns>
    Task<RepoSyncResult> SynchronizeAsync(GitRepository repo, GitAccount? account, string? branch = null, CancellationToken ct = default);
    Task<RepoSyncResult> SynchronizeAsync(
        GitRepository repo,
        GitAccount? account,
        string? branch,
        RepoSyncOptions options,
        CancellationToken ct = default);
    Task DeleteCacheAsync(GitRepository repo, CancellationToken ct = default);
    Task DeleteCacheAsync(string path, CancellationToken ct = default);
    string GetRemoteUrl(GitRepository repo, GitAccount? account);
}

internal sealed class RepoCacheManager(IGitCliRepository gitCli, ILogger<RepoCacheManager> logger) : IRepoCacheManager
{
    private readonly ConcurrentDictionary<Guid, RepoLockEntry> _repoLocks = new();
    internal int ActiveLockCount => _repoLocks.Count;

    public Task<RepoSyncResult> SynchronizeAsync(
        GitRepository repo,
        GitAccount? account,
        string? branch = null,
        CancellationToken ct = default)
        => SynchronizeAsync(repo, account, branch, RepoSyncOptions.Default, ct);

    public async Task<RepoSyncResult> SynchronizeAsync(
        GitRepository repo,
        GitAccount? account,
        string? branch,
        RepoSyncOptions options,
        CancellationToken ct = default)
    {
        using var repoLock = await AcquireRepoLockAsync(repo.Id, ct);

        var targetPath = ApplicationStoragePaths.GetRepositoryCachePath(repo);
        var url = GetRemoteUrl(repo, account);
        var syncBranch = string.IsNullOrWhiteSpace(branch) ? repo.DefaultBranch ?? "main" : branch;
        GitOperation operation = GitOperation.Pull;

        // Ensure Local Source Exists
        if (!Directory.Exists(Path.Combine(targetPath, ".git")))
        {
            await EnsureDeletedAsync(targetPath, ct);
            var cloneResult = await gitCli.CloneAsync(url, targetPath, syncBranch, account, ct);

            operation = GitOperation.Clone;

            if (cloneResult.IsFailure(out var error))
                return new RepoSyncResult(Operation: operation, Error: error.Message);

        }
        else
        {
            // Fetch the requested branch without relying on the current working-tree branch.
            var fetchResult = await gitCli.FetchAsync(targetPath, syncBranch, account, ct);
            if (fetchResult.IsFailure(out var error))
                return new RepoSyncResult(Operation: operation, Error: error.Message);

            var resetResult = await gitCli.ResetWorkingTreeAsync(targetPath, syncBranch, ct);
            if (resetResult.IsFailure(out error))
                return new RepoSyncResult(Operation: operation, Error: error.Message);
        }

        if (options.ExecuteHooks)
        {
            var hookResult = await ExecuteHooksInternalAsync(repo, targetPath, operation, ct);
            if (hookResult.IsFailure(out var hookError))
                return new RepoSyncResult(Operation: operation, Error: hookError.Message);
        }

        // Resolve the SHA for the state tracker
        var hashResult = await gitCli.ResolveSnapshotCommitAsync(targetPath, syncBranch, ct);
        if (hashResult.IsFailure(out var hashError, out var hash))
            return new RepoSyncResult(Operation: operation, Error: hashError.Message);

        return new RepoSyncResult(operation, hash, Success: true, CachePath: targetPath);
    }

    public Task DeleteCacheAsync(string path, CancellationToken ct = default)
        => EnsureDeletedAsync(path, ct);

    public async Task DeleteCacheAsync(GitRepository repo, CancellationToken ct = default)
    {
        using var repoLock = await AcquireRepoLockAsync(repo.Id, ct);

        await EnsureDeletedAsync(
            ApplicationStoragePaths.GetRepositoryCachePath(repo),
            ct);
    }

    private async ValueTask<RepoLockLease> AcquireRepoLockAsync(
        Guid repositoryId,
        CancellationToken cancellationToken)
    {
        while (true)
        {
            var entry = _repoLocks.GetOrAdd(repositoryId, static _ => new RepoLockEntry());
            lock (entry)
            {
                if (entry.Removed)
                    continue;
                entry.ReferenceCount++;
            }

            try
            {
                await entry.Semaphore.WaitAsync(cancellationToken);
                return new RepoLockLease(this, repositoryId, entry);
            }
            catch
            {
                ReleaseRepoLock(repositoryId, entry, releaseSemaphore: false);
                throw;
            }
        }
    }

    private void ReleaseRepoLock(
        Guid repositoryId,
        RepoLockEntry entry,
        bool releaseSemaphore)
    {
        if (releaseSemaphore)
            entry.Semaphore.Release();

        var dispose = false;
        lock (entry)
        {
            entry.ReferenceCount--;
            if (entry.ReferenceCount == 0)
            {
                entry.Removed = true;
                dispose = _repoLocks.TryRemove(
                    new KeyValuePair<Guid, RepoLockEntry>(repositoryId, entry));
            }
        }

        if (dispose)
            entry.Semaphore.Dispose();
    }

    private sealed class RepoLockEntry
    {
        public SemaphoreSlim Semaphore { get; } = new(1, 1);
        public int ReferenceCount { get; set; }
        public bool Removed { get; set; }
    }

    private readonly struct RepoLockLease(
        RepoCacheManager owner,
        Guid repositoryId,
        RepoLockEntry entry) : IDisposable
    {
        public void Dispose()
            => owner.ReleaseRepoLock(repositoryId, entry, releaseSemaphore: true);
    }

    private async Task<Result> ExecuteHooksInternalAsync(GitRepository repo, string repoRoot, GitOperation operation, CancellationToken ct)
    {
        // Run OnClone hooks only if we just cloned
        if (operation == GitOperation.Clone && repo.OnClone is not null && repo.OnClone?.Commands.Count > 0)
        {
            logger.LogInformation("Executing OnClone hooks for {RepoName}", repo.Name);
            var result = await RunCommandListAsync(repoRoot, repo.OnClone, ct);
            if (result.IsFailure()) return result;
        }

        // Run OnPull hooks every time (including after a fresh clone)
        if (repo.OnPull is not null && repo.OnPull.Commands.Count > 0)
        {
            logger.LogInformation("Executing OnPull hooks for {RepoName}", repo.Name);
            var result = await RunCommandListAsync(repoRoot, repo.OnPull, ct);
            if (result.IsFailure()) return result;
        }

        return Result.Success();
    }

    private async Task<Result> RunCommandListAsync(string repoRoot, RepoCommand? hook, CancellationToken ct)
    {
        if (hook is null) return Result.Success();

        var pathResult = ResolveHookWorkingDirectory(repoRoot, hook.Path);
        if (pathResult.IsFailure(out var error, out var executionDir))
            return Result.Failure(error);

        foreach (var command in hook.Commands)
        {
            var result = await gitCli.ExecuteShellCommandAsync(executionDir, command, ct);
            if (result.IsFailure()) return result;
        }

        return Result.Success();
    }

    internal static Result<string> ResolveHookWorkingDirectory(string repoRoot, string hookPath)
    {
        if (Path.IsPathRooted(hookPath))
            return Result.Failure<string>($"Hook path '{hookPath}' must be relative to the repository root.");

        var normalizedRoot = Path.GetFullPath(repoRoot);
        var executionDirectory = Path.GetFullPath(Path.Combine(normalizedRoot, hookPath));
        var comparison = OperatingSystem.IsWindows()
            ? StringComparison.OrdinalIgnoreCase
            : StringComparison.Ordinal;
        var rootWithSeparator = Path.EndsInDirectorySeparator(normalizedRoot)
            ? normalizedRoot
            : normalizedRoot + Path.DirectorySeparatorChar;

        if (!string.Equals(executionDirectory, normalizedRoot, comparison)
            && !executionDirectory.StartsWith(rootWithSeparator, comparison))
        {
            return Result.Failure<string>($"Hook path '{hookPath}' is outside the repository root.");
        }

        if (ContainsLinkOutsideRoot(normalizedRoot, executionDirectory, comparison))
            return Result.Failure<string>($"Hook path '{hookPath}' points outside the repository root.");

        return Result.Success(executionDirectory);
    }

    private static bool ContainsLinkOutsideRoot(
        string normalizedRoot,
        string executionDirectory,
        StringComparison comparison)
    {
        var canonicalRoot = ResolveDirectoryLink(normalizedRoot);
        var current = canonicalRoot;
        var relativePath = Path.GetRelativePath(normalizedRoot, executionDirectory);

        foreach (var segment in relativePath.Split(
                     [Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar],
                     StringSplitOptions.RemoveEmptyEntries))
        {
            current = ResolveDirectoryLink(Path.Combine(current, segment));
            if (!IsWithinRoot(canonicalRoot, current, comparison))
                return true;
        }

        return false;
    }

    private static string ResolveDirectoryLink(string path)
    {
        var fullPath = Path.GetFullPath(path);
        if (!Directory.Exists(fullPath))
            return fullPath;

        try
        {
            return new DirectoryInfo(fullPath).ResolveLinkTarget(returnFinalTarget: true) is { } target
                ? Path.GetFullPath(target.FullName)
                : fullPath;
        }
        catch (IOException)
        {
            return fullPath;
        }
        catch (UnauthorizedAccessException)
        {
            return fullPath;
        }
    }

    private static bool IsWithinRoot(
        string normalizedRoot,
        string path,
        StringComparison comparison)
    {
        if (string.Equals(path, normalizedRoot, comparison))
            return true;

        var rootWithSeparator = Path.EndsInDirectorySeparator(normalizedRoot)
            ? normalizedRoot
            : normalizedRoot + Path.DirectorySeparatorChar;
        return path.StartsWith(rootWithSeparator, comparison);
    }

    private async Task EnsureDeletedAsync(string path, CancellationToken ct)
    {
        if (!Directory.Exists(path)) return;

        const int maxRetries = 3;
        for (var i = 0; i < maxRetries; i++)
        {
            try
            {
                ct.ThrowIfCancellationRequested();
                Directory.Delete(path, recursive: true);
                return;
            }
            catch (IOException)
            {
                if (i == maxRetries - 1) throw;

                logger.LogWarning("Retry {Count}: Failed to delete {Path}. Handle might be busy.", i + 1, path);
                await Task.Delay(200 * (i + 1), ct);
            }
        }
    }

    public string GetRemoteUrl(GitRepository repo, GitAccount? account)
    {
        if (repo.Url.StartsWith("http", StringComparison.OrdinalIgnoreCase) ||
            repo.Url.StartsWith("git@", StringComparison.OrdinalIgnoreCase) ||
            repo.Url.StartsWith("file://", StringComparison.OrdinalIgnoreCase))
        {
            return repo.Url;
        }

        if (account == null)
        {
            throw new InvalidOperationException(
                $"Repository {repo.Id} requires a complete URL when no Git account is selected.");
        }

        return $"https://{account.Domain}/{repo.Url.TrimStart('/')}";
    }
}

internal sealed record RepoSyncOptions(bool ExecuteHooks = true)
{
    public static readonly RepoSyncOptions Default = new();
    public static readonly RepoSyncOptions WithoutHooks = new(ExecuteHooks: false);
}

internal sealed record RepoSyncResult(
    GitOperation Operation,
    string? Hash = null,
    bool? Success = false,
    string? Error = null,
    string? CachePath = null);
