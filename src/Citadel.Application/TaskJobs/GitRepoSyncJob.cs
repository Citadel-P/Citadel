using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal class GitRepoSyncJob(
    IDbWorkQueue dbWorkQueue,
    IServiceScopeFactory scopeFactory,
    IGitCliRepository gitCliRepository,
    IRepoCacheManager repoCacheManager,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    ChannelReader<GitRepoSyncRequest> gitSyncReader,
    IGitRepositoryStreamManager gitRepoStreamManager,
    ILogger<GitRepoSyncJob> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            await foreach (var request in gitSyncReader.ReadAllAsync(stoppingToken))
            {
                try
                {
                    var repo = await GetGitRepo(request.RepoId, stoppingToken);
                    if (repo is null)
                    {
                        logger.LogWarning("Repo with ID {RepoId} not found for sync", request.RepoId);
                        continue;
                    }

                    var connectionResult = await gitCliRepository.TestConnectionAsync(repo.Url, repo.GitAccount, stoppingToken);

                    if (connectionResult.IsFailure(out var error))
                    {
                        await dbWorkQueue.EnqueueAsync(new GitRepoSyncFailedWorkItem(
                            gitRepoStreamManager,
                            notificationQueue,
                            activityHub, 
                            logger,
                            GitOperation.Authenticate,
                            repo, 
                            error.Message), stoppingToken);
                    }
                    else
                    {
                        var syncResult = await repoCacheManager.SynchronizeAsync(repo, repo.GitAccount, stoppingToken);
                        if (syncResult.Success != null &&!syncResult.Success.Value)
                        {
                            await dbWorkQueue.EnqueueAsync(new GitRepoSyncFailedWorkItem(
                                gitRepoStreamManager,
                                notificationQueue,
                                activityHub,
                                logger,
                                syncResult.Operation,
                                repo,
                                syncResult.Error ?? "Unknown error"), stoppingToken);
                        }
                        else
                        {
                            await dbWorkQueue.EnqueueAsync(new GitRepoSyncSuccessWorkItem(
                                gitRepoStreamManager,
                                notificationQueue,
                                activityHub,
                                syncResult.Operation,
                                syncResult.Hash,
                                repo), stoppingToken);
                        }
                    }
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Background sync failed for repo {RepoId}", request.RepoId);
                }
            }
        }
        catch (OperationCanceledException) { /** Nope */ }
    }

    private async Task<GitRepository?> GetGitRepo(Guid repoId, CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await unitOfWork.GitRepositories.GetWithAccountAsync(repoId, stoppingToken);
    }
}

internal sealed class GitRepoSyncSuccessWorkItem(
    IGitRepositoryStreamManager streamManager,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    GitOperation gitOperation,
    string commitHash,
    GitRepository repo) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        // Update repo status
        repo.ReleaseProcessing(GitReposStatus.Healthy);
        await uow.GitRepositories.UpdateAsync(repo, cancellationToken);

        // Add activity
        var syncResult = new RepoSyncResultSnapshot(commitHash, null);
        var activity = new ActivityEvent(
            actorId: Constants.SystemId,
            resourceId: repo.Id,
            platformId: null,
            resourceName: repo.Name,
            eventType: gitOperation == GitOperation.Pull 
                ? ActivityEventType.GitRepoPulled 
                : ActivityEventType.GitRepoCloned,
            status: ActivityStatus.Success,
            info: gitOperation == GitOperation.Pull  
                ? new GitRepoPulled(repo.ToSnapshot(), syncResult)
                : new GitRepoCloned(repo.ToSnapshot(), syncResult)
            );

        await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await uow.CommitAsync(cancellationToken);

        // Notify clients
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, cancellationToken)), cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(streamManager, repo), cancellationToken);
    }
}

internal sealed class GitRepoSyncFailedWorkItem(
    IGitRepositoryStreamManager streamManager,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    ILogger<GitRepoSyncJob> logger,
    GitOperation gitOperation,
    GitRepository repo,
    string errorMessage
    ) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        logger.LogError(errorMessage);

        // Update repo status
        repo.ReleaseProcessing(GitReposStatus.Degraded);
        await uow.GitRepositories.UpdateAsync(repo, cancellationToken);

        // Add activity
        var syncResult = new RepoSyncResultSnapshot(null, errorMessage);
        var activity = new ActivityEvent(
            actorId: Constants.SystemId,
            resourceId: repo.Id,
            platformId: null,
            resourceName: repo.Name,
            eventType: gitOperation == GitOperation.Pull 
                ? ActivityEventType.GitRepoPulled 
                : ActivityEventType.GitRepoCloned,
            status: ActivityStatus.Failure,
            info: gitOperation == GitOperation.Pull 
                ? new GitRepoPulled(repo.ToSnapshot(), syncResult)
                : new GitRepoCloned(repo.ToSnapshot(), syncResult)
            );

        await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await uow.CommitAsync(cancellationToken);

        // Notify clients
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, cancellationToken)), cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(streamManager, repo), cancellationToken);
    }
}

internal class GitRepoNotificationWorkItem(IGitRepositoryStreamManager gitRepoHub, GitRepository repo, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => gitRepoHub.SendGitRepoInfo(repo, action);
}

internal readonly record struct GitRepoSyncRequest(Guid RepoId);
