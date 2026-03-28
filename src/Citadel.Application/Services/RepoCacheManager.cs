using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;

namespace Application.Services;

/// <summary>
/// Manages the lifecycle and synchronization of locally cached Git repositories.
/// </summary>
public interface IRepoCacheManager
{
    /// <summary>
    /// Ensures the local bare repository is cloned and updated.
    /// </summary>
    /// <returns>The resolved commit hash (SHA)</returns>
    Task<Result<string>> SynchronizeAsync(GitRepository repo, GitAccount? account, CancellationToken ct = default);
}

internal sealed class RepoCacheManager(IGitCliRepository gitCli, ILogger<RepoCacheManager> logger) : IRepoCacheManager
{
    private readonly ConcurrentDictionary<Guid, SemaphoreSlim> _repoLocks = new();

    public async Task<Result<string>> SynchronizeAsync(GitRepository repo, GitAccount? account, CancellationToken ct = default)
    {
        var semaphore = _repoLocks.GetOrAdd(repo.Id, _ => new SemaphoreSlim(1, 1));
        await semaphore.WaitAsync(ct);

        try
        {
            var targetPath = repo.GetCachePath();
            var url = GetRemoteUrl(repo, account);
            var branch = repo.DefaultBranch ?? "main";
            bool isNewClone = false;

            // Ensure Local Source Exists
            if (!Directory.Exists(Path.Combine(targetPath, ".git")))
            {
                await EnsureDeletedAsync(targetPath, ct);
                var cloneResult = await gitCli.CloneAsync(url, targetPath, branch, account, ct);

                if (cloneResult.IsFailure(out var error))
                    return Result.Failure<string>(new InternalServerError(error.Message));

                isNewClone = true;
            }
            else
            {
                // Pull latest changes
                var pullResult = await gitCli.PullAsync(targetPath, branch, account, ct);
                if (pullResult.IsFailure(out var error))
                    return Result.Failure<string>(new InternalServerError(error.Message));
            }

            // Execute System Hooks
            var hookResult = await ExecuteHooksInternalAsync(repo, targetPath, isNewClone, ct);
            if (hookResult.IsFailure(out var hookError))
                return Result.Failure<string>(new InternalServerError(hookError.Message));

            // Resolve the SHA for the state tracker
            return await gitCli.ResolveSnapshotCommitAsync(targetPath, branch, ct);
        }
        finally
        {
            semaphore.Release();
        }
    }

    private async Task<Result> ExecuteHooksInternalAsync(GitRepository repo, string repoRoot, bool isNewClone, CancellationToken ct)
    {
        // Run OnClone hooks only if we just cloned
        if (isNewClone && repo.OnClone.Count > 0)
        {
            logger.LogInformation("Executing OnClone hooks for {RepoName}", repo.Name);
            var result = await RunCommandListAsync(repoRoot, repo.OnClone, ct);
            if (result.IsFailure()) return result;
        }

        // Run OnPull hooks every time (including after a fresh clone)
        if (repo.OnPull.Count > 0)
        {
            logger.LogInformation("Executing OnPull hooks for {RepoName}", repo.Name);
            var result = await RunCommandListAsync(repoRoot, repo.OnPull, ct);
            if (result.IsFailure()) return result;
        }

        return Result.Success();
    }

    private async Task<Result> RunCommandListAsync(string repoRoot, IEnumerable<RepoCommand> hooks, CancellationToken ct)
    {
        foreach (var hook in hooks)
        {
            // Combine repo root with the command's relative path
            var executionDir = Path.GetFullPath(Path.Combine(repoRoot, hook.Path));

            // Security: Prevent Directory Traversal
            if (!executionDir.StartsWith(repoRoot, StringComparison.OrdinalIgnoreCase))
                return Result.Failure($"Security Violation: Hook path '{hook.Path}' is outside repo root.");

            var result = await gitCli.ExecuteShellCommandAsync(executionDir, hook.Command, ct);
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

    private static string GetRemoteUrl(GitRepository repo, GitAccount? account)
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
