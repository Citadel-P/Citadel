using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class GitRepositoryPollingJob(
    IServiceScopeFactory scopeFactory,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
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
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var repos = await uow.GitRepositories.GetAllAsync(cancellationToken);
        var subscriptions = await uow.Stacks.GetGitStackBranchSubscriptionsAsync(cancellationToken);
        var now = DateTime.UtcNow;

        var requests = new HashSet<(Guid RepoId, string Branch)>();

        foreach (var repo in repos)
        {
            if (repo.SyncMode != GitRepositorySyncMode.PullInterval)
                continue;

            var branches = new HashSet<string>(StringComparer.Ordinal);

            if (!string.IsNullOrWhiteSpace(repo.DefaultBranch))
            {
                branches.Add(repo.DefaultBranch);
            }

            foreach (var subscription in subscriptions.Where(x => x.GitRepositoryId == repo.Id))
            {
                if (!string.IsNullOrWhiteSpace(subscription.Branch))
                {
                    branches.Add(subscription.Branch);
                }
            }

            var refs = (await uow.GitRepositories.GetRefsByRepositoryIdAsync(repo.Id, cancellationToken))
                .ToDictionary(x => x.Branch, StringComparer.Ordinal);

            var interval = TimeSpan.FromMinutes(repo.SyncIntervalMinutes ?? 5);
            foreach (var branch in branches)
            {
                if (!refs.TryGetValue(branch, out var branchRef)
                    || branchRef.LastSyncedAt.Add(interval) <= now)
                {
                    requests.Add((repo.Id, branch));
                }
            }
        }

        foreach (var (repoId, branch) in requests)
        {
            try
            {
                await gitSyncWriter.WriteAsync(
                    new GitRepoSyncRequest(repoId, branch, GitRepoSyncTrigger.Poll),
                    cancellationToken);
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                logger.LogWarning(ex, "Failed to enqueue git repository poll sync for {RepoId} branch {Branch}", repoId, branch);
            }
        }
    }
}
