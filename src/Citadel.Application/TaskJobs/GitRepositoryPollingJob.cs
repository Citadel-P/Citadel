using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class GitRepositoryPollingJob(
    IServiceScopeFactory scopeFactory,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    GitRepoSyncInFlightTracker inFlightTracker,
    ILogger<GitRepositoryPollingJob> logger) : BackgroundService
{
    private static readonly TimeSpan PollInterval = TimeSpan.FromMinutes(1);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            await PollOnceAsync(stoppingToken);

            using var timer = new PeriodicTimer(PollInterval);
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                await PollOnceAsync(stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private async Task PollOnceAsync(CancellationToken cancellationToken)
    {
        var requests = new HashSet<(Guid RepoId, string Branch)>();

        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repos = (await uow.GitRepositories.GetAllAsync(cancellationToken))
                .Where(static repo => repo.SyncMode == GitRepositorySyncMode.PullInterval)
                .ToList();
            var repositoryIds = repos.Select(static repo => repo.Id).ToArray();
            var repositoryIdSet = repositoryIds.ToHashSet();
            var subscriptions = (await uow.Stacks.GetGitStackBranchSubscriptionsAsync(cancellationToken))
                .Where(subscription => repositoryIdSet.Contains(subscription.GitRepositoryId))
                .GroupBy(static subscription => subscription.GitRepositoryId)
                .ToDictionary(static group => group.Key, static group => group.ToArray());
            var refs = (await uow.GitRepositories.GetRefsByRepositoryIdsAsync(repositoryIds, cancellationToken))
                .GroupBy(static gitRef => gitRef.GitRepositoryId)
                .ToDictionary(
                    static group => group.Key,
                    static group => group.ToDictionary(gitRef => gitRef.Branch, StringComparer.Ordinal));
            var now = DateTime.UtcNow;

            foreach (var repo in repos)
            {
                var branches = new HashSet<string>(StringComparer.Ordinal);

                if (!string.IsNullOrWhiteSpace(repo.DefaultBranch))
                    branches.Add(repo.DefaultBranch);

                if (subscriptions.TryGetValue(repo.Id, out var repositorySubscriptions))
                {
                    foreach (var subscription in repositorySubscriptions)
                    {
                        if (!string.IsNullOrWhiteSpace(subscription.Branch))
                            branches.Add(subscription.Branch);
                    }
                }

                var repositoryRefs = refs.GetValueOrDefault(repo.Id);
                var interval = TimeSpan.FromMinutes(repo.SyncIntervalMinutes ?? 5);
                foreach (var branch in branches)
                {
                    if (repositoryRefs is null
                        || !repositoryRefs.TryGetValue(branch, out var branchRef)
                        || branchRef.LastSyncedAt.Add(interval) <= now)
                    {
                        requests.Add((repo.Id, branch));
                    }
                }
            }
        }

        foreach (var (repoId, branch) in requests)
        {
            if (!inFlightTracker.TryAdd(repoId, branch))
                continue;

            try
            {
                await gitSyncWriter.WriteAsync(
                    new GitRepoSyncRequest(repoId, branch, GitRepoSyncTrigger.Poll),
                    cancellationToken);
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                inFlightTracker.Remove(repoId, branch);
                logger.LogWarning(ex, "Failed to enqueue git repository poll sync for {RepoId} branch {Branch}", repoId, branch);
            }
            catch
            {
                inFlightTracker.Remove(repoId, branch);
                throw;
            }
        }
    }
}

internal sealed class GitRepoSyncInFlightTracker
{
    private readonly ConcurrentDictionary<(Guid RepoId, string Branch), byte> entries = new();

    public bool TryAdd(Guid repoId, string branch)
        => entries.TryAdd((repoId, NormalizeBranch(branch)), 0);

    public void Remove(Guid repoId, string? branch)
    {
        if (!string.IsNullOrWhiteSpace(branch))
            entries.TryRemove((repoId, NormalizeBranch(branch)), out _);
    }

    private static string NormalizeBranch(string branch) => branch.Trim().ToUpperInvariant();
}
