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
    Task DeleteCacheAsync(GitRepository repo, CancellationToken ct = default);
    Task DeleteCacheAsync(string path, CancellationToken ct = default);
    string GetRemoteUrl(GitRepository repo, GitAccount? account);
}

internal sealed class RepoCacheManager(IGitCliRepository gitCli, ILogger<RepoCacheManager> logger) : IRepoCacheManager
{
    private readonly ConcurrentDictionary<Guid, SemaphoreSlim> _repoLocks = new();

    public async Task<RepoSyncResult> SynchronizeAsync(GitRepository repo, GitAccount? account, string? branch = null, CancellationToken ct = default)
    {
        var semaphore = _repoLocks.GetOrAdd(repo.Id, _ => new SemaphoreSlim(1, 1));
        await semaphore.WaitAsync(ct);

        try
        {
            var targetPath = repo.GetCachePath();
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
            }

            // Execute SystemMessage Hooks
            var hookResult = await ExecuteHooksInternalAsync(repo, targetPath, operation, ct);
            if (hookResult.IsFailure(out var hookError))
                return new RepoSyncResult(Operation: operation, Error: hookError.Message);

            // Resolve the SHA for the state tracker
            var hashResult = await gitCli.ResolveSnapshotCommitAsync(targetPath, syncBranch, ct);
            if (hashResult.IsFailure(out var hashError, out var hash))
                return new RepoSyncResult(Operation: operation, Error: hashError.Message);

            return new RepoSyncResult(operation, hash, Success: true);
        }
        finally
        {
            semaphore.Release();
        }
    }

    public Task DeleteCacheAsync(string path, CancellationToken ct = default)
        => EnsureDeletedAsync(path, ct);

    public async Task DeleteCacheAsync(GitRepository repo, CancellationToken ct = default)
    {
        var semaphore = _repoLocks.GetOrAdd(repo.Id, _ => new SemaphoreSlim(1, 1));
        await semaphore.WaitAsync(ct);

        try
        {
            await EnsureDeletedAsync(repo.GetCachePath(), ct);
        }
        finally
        {
            semaphore.Release();
        }
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

        // Combine repo root with the command's relative path
        var executionDir = Path.GetFullPath(Path.Combine(repoRoot, hook.Path));

        // GrpcRequestMetadata: Prevent Directory Traversal
        if (!executionDir.StartsWith(repoRoot, StringComparison.OrdinalIgnoreCase))
            return Result.Failure($"Security Violation: Hook path '{hook.Path}' is outside repo root.");

        foreach (var command in hook.Commands)
        {
            var result = await gitCli.ExecuteShellCommandAsync(executionDir, command, ct);
            if (result.IsFailure()) return result;
        }

        return Result.Success();
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
            repo.Url.StartsWith("git@", StringComparison.OrdinalIgnoreCase))
        {
            return repo.Url;
        }

        if (account == null)
            throw new InvalidOperationException($"Auth required for {repo.Id} but no account provided.");

        return $"https://{account.Domain}/{repo.Url.TrimStart('/')}";
    }
}

internal sealed record RepoSyncResult(
    GitOperation Operation,
    string? Hash = null,
    bool? Success = false,
    string? Error = null);
