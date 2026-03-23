using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;

namespace Application.Services;

/// <summary>
/// Manages the lifecycle and synchronization of locally cached Git repositories.
///
/// This component ensures that a repository is available locally as a valid,
/// up-to-date bare repository and that a deterministic commit snapshot can be
/// resolved for a given branch.
///
/// Responsibilities:
/// - Maintain a local bare repository cache per Git repository
/// - Synchronize the cache with the remote source (clone or fetch)
/// - Recover automatically from corrupted or inconsistent repository states
/// - Resolve a deterministic commit hash for a branch using snapshot semantics
/// - Serialize access per repository to prevent concurrent mutations
///
/// Synchronization strategy:
/// - If the repository does not exist or is invalid → full re-clone
/// - If the repository exists → shallow fetch of the target branch
/// - If fetch fails → fallback to full re-clone
/// - If commit resolution fails → final fallback to full re-clone
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

            if (!await IsValidBareRepo(targetPath, branch, ct))
            {
                await EnsureDeletedAsync(targetPath, ct);

                var cloneResult = await gitCli.CloneBareAsync(url, targetPath, account, ct);
                if (cloneResult.IsFailure(out var error))
                    return Result.Failure<string>(new InternalServerError(error.Message));
            }
            else
            {
                var fetchResult = await gitCli.FetchBranchSnapshotAsync(targetPath, branch, account, ct);

                if (fetchResult.IsFailure(out var error))
                {
                    logger.LogWarning("Fetch failed for {RepoId}: {Error}. Re-cloning...", repo.Id, error.Message);
                    await EnsureDeletedAsync(targetPath, ct);

                    var recloneResult = await gitCli.CloneBareAsync(url, targetPath, account, ct);
                    if (recloneResult.IsFailure(out var recloneError))
                        return Result.Failure<string>(new InternalServerError(recloneError.Message));
                }
            }

            var commitResult = await gitCli.ResolveSnapshotCommitAsync(targetPath, branch, ct);

            if (commitResult.IsFailure(out var resolveError))
            {
                logger.LogWarning("Resolve failed for {RepoId}: {Error}. Re-cloning...", repo.Id, resolveError.Message);
                await EnsureDeletedAsync(targetPath, ct);

                var lastResort = await gitCli.CloneBareAsync(url, targetPath, account, ct);
                if (lastResort.IsFailure(out var lastResortError))
                    return Result.Failure<string>(new InternalServerError(lastResortError.Message));

                commitResult = await gitCli.ResolveSnapshotCommitAsync(targetPath, branch, ct);
            }

            return commitResult;
        }
        finally
        {
            semaphore.Release();
        }
    }

    private async Task<bool> IsValidBareRepo(string path, string branch, CancellationToken ct)
    {
        if (!Directory.Exists(path)) return false;

        var result = await gitCli.ResolveSnapshotCommitAsync(path, branch, ct);
        return result.IsSuccess();
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
