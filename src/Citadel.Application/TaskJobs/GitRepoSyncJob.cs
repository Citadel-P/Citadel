using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
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
    IStackStreamManager stackStreamManager,
    IAlertService alertService,
    IApplyStackService applyStackService,
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

                    var branch = string.IsNullOrWhiteSpace(request.Branch)
                        ? repo.DefaultBranch ?? "main"
                        : request.Branch;

                    var connectionResult = await gitCliRepository.TestConnectionAsync(
                        repoCacheManager.GetRemoteUrl(repo, repo.GitAccount),
                        repo.GitAccount,
                        stoppingToken);

                    if (connectionResult.IsFailure(out var error))
                    {
                        await dbWorkQueue.EnqueueAsync(new GitRepoSyncFailedWorkItem(
                            gitRepoStreamManager,
                            notificationQueue,
                            activityHub, 
                            GitOperation.Authenticate,
                            repo, 
                            branch,
                            error.Message,
                            request.Trigger,
                            BranchScopedFailure: false), stoppingToken);
                    }
                    else
                    {
                        var syncResult = await repoCacheManager.SynchronizeAsync(repo, repo.GitAccount, branch, stoppingToken);
                        if ((syncResult.Success != null && !syncResult.Success.Value) || string.IsNullOrWhiteSpace(syncResult.Hash))
                        {
                            await dbWorkQueue.EnqueueAsync(new GitRepoSyncFailedWorkItem(
                                gitRepoStreamManager,
                                notificationQueue,
                                activityHub,
                                syncResult.Operation,
                                repo,
                                branch,
                                syncResult.Error ?? "Repository sync did not resolve a commit.",
                                request.Trigger,
                                BranchScopedFailure: true), stoppingToken);
                        }
                        else
                        {
                            await dbWorkQueue.EnqueueAsync(new GitRepoSyncSuccessWorkItem(
                                gitRepoStreamManager,
                                stackStreamManager,
                                notificationQueue,
                                activityHub,
                                alertService,
                                applyStackService,
                                syncResult.Operation,
                                syncResult.Hash!,
                                branch,
                                request.Trigger,
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
    IStackStreamManager stackStreamManager,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IAlertService alertService,
    IApplyStackService applyStackService,
    GitOperation gitOperation,
    string commitHash,
    string branch,
    GitRepoSyncTrigger trigger,
    GitRepository repo) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var primaryRepositorySync = GitRepoSyncScope.IsPrimaryRepositorySync(repo, branch, trigger);

        // Update repo status
        if (primaryRepositorySync)
        {
            repo.ReleaseProcessing(GitReposStatus.Healthy);
            await uow.GitRepositories.UpdateAsync(repo, cancellationToken);
        }

        await uow.GitRepositories.UpsertRefAsync(
            new GitRepositoryRef(repo.Id, branch, commitHash, GitReposStatus.Healthy),
            cancellationToken);

        // Add activity
        ActivityEvent? activity = null;
        if (primaryRepositorySync)
        {
            var repoSnapshot = repo.ToSnapshot(resolvedCommitSha: commitHash);
            var syncResult = new RepoSyncResultSnapshot(commitHash, null);
            activity = new ActivityEvent(
                actorId: Constants.SystemId,
                resourceId: repo.Id,
                platformId: null,
                resourceName: repo.Name,
                eventType: gitOperation == GitOperation.Pull
                    ? ActivityEventType.GitRepoPulled
                    : ActivityEventType.GitRepoCloned,
                status: ActivityStatus.Success,
                info: gitOperation == GitOperation.Pull
                    ? new GitRepoPulled(repoSnapshot, syncResult)
                    : new GitRepoCloned(repoSnapshot, syncResult)
                );

            await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }

        var stackUpdates = await UpdateLinkedGitStacksAsync(uow, cancellationToken);

        await uow.CommitAsync(cancellationToken);

        // Notify clients
        if (activity is not null)
        {
            repo.AssignActivityEvent(activity);
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, cancellationToken)), cancellationToken);
            await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(streamManager, repo), cancellationToken);
        }

        foreach (var update in stackUpdates)
        {
            await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackStreamManager, update.Stack), cancellationToken);

            if (update.AvailableActivity is not null)
            {
                await notificationQueue.EnqueueAsync(
                    new ActivityNotificationWorkItem(activityHub, await update.AvailableActivity.AssignActor(uow, cancellationToken)),
                    cancellationToken);
            }

            if (update.ShouldNotify)
            {
                await ProcessGitStackAlertAsync(AlertType.StackGitUpdateAvailable, update, failed: false, null, cancellationToken);
                continue;
            }

            if (update.ShouldAutoDeploy)
            {
                var (success, error) = await TryAutoDeployAsync(update.Stack.Id, cancellationToken);
                if (success)
                {
                    var successActivity = CreateGitStackActivity(
                        update,
                        ActivityEventType.StackGitAutoUpdated,
                        ActivityStatus.Success,
                        new StackGitAutoUpdated(repo.Name, update.Branch, update.CurrentCommitSha, update.RemoteCommitSha));

                    await uow.ActivityEventRepository.AddAsync(successActivity, cancellationToken);
                    await uow.CommitAsync(cancellationToken);
                    await notificationQueue.EnqueueAsync(
                        new ActivityNotificationWorkItem(activityHub, await successActivity.AssignActor(uow, cancellationToken)),
                        cancellationToken);
                    await ProcessGitStackAlertAsync(AlertType.StackGitAutoUpdated, update, failed: false, null, cancellationToken);
                }
                else
                {
                    var reason = error ?? "Auto-deploy failed.";
                    var failureActivity = CreateGitStackActivity(
                        update,
                        ActivityEventType.StackGitAutoDeployFailed,
                        ActivityStatus.Failure,
                        new StackGitAutoDeployFailed(repo.Name, update.Branch, update.CurrentCommitSha, update.RemoteCommitSha, reason));

                    await uow.ActivityEventRepository.AddAsync(failureActivity, cancellationToken);
                    await uow.CommitAsync(cancellationToken);
                    await notificationQueue.EnqueueAsync(
                        new ActivityNotificationWorkItem(activityHub, await failureActivity.AssignActor(uow, cancellationToken)),
                        cancellationToken);
                    await ProcessGitStackAlertAsync(AlertType.StackGitAutoDeployFailed, update, failed: true, reason, cancellationToken);
                }
            }
        }
    }

    private async Task<List<LinkedGitStackUpdate>> UpdateLinkedGitStacksAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var linkedStacks = await uow.Stacks.GetBranchTrackingGitStacksAsync(repo.Id, branch, cancellationToken);
        var updates = new List<LinkedGitStackUpdate>();
        var now = DateTime.UtcNow;

        foreach (var stack in linkedStacks)
        {
            if (stack.CurrentStackRelease?.Spec is not GitStack gitStack)
                continue;

            if (gitStack.UpdateBehavior == StackUpdateBehavior.Disabled)
                continue;

            var source = stack.CurrentStackRelease.Source;
            if (source is null
                || source.SourceType != StackSource.Git
                || source.GitRepositoryId != repo.Id
                || !string.Equals(source.Branch, branch, StringComparison.Ordinal)
                || string.IsNullOrWhiteSpace(source.ResolvedCommitSha))
            {
                continue;
            }

            var currentCommit = source.ResolvedCommitSha;
            var remoteCommit = commitHash;
            var updateAvailable = !string.Equals(currentCommit, remoteCommit, StringComparison.OrdinalIgnoreCase);

            var currentState = stack.StackUpdateState as GitStackUpdateState
                ?? new GitStackUpdateState(
                    new RecreateStackOnNewImageState([]),
                    new RecreateStackOnNewCommitState(currentCommit, null, DateTime.MinValue));

            var previousCommitState = currentState.RecreateStackOnNewCommitState;
            var alreadyReported = updateAvailable
                && string.Equals(previousCommitState.RemoteCommitSha, remoteCommit, StringComparison.OrdinalIgnoreCase);

            stack.SetStackUpdateState(currentState with
            {
                RecreateStackOnNewCommitState = new RecreateStackOnNewCommitState(
                    CurrentCommitSha: currentCommit,
                    RemoteCommitSha: updateAvailable ? remoteCommit : null,
                    LastCheckedAt: now)
            });

            await uow.Stacks.UpdateAsync(stack, cancellationToken);

            if (!updateAvailable || alreadyReported)
                continue;

            var shouldNotify = gitStack.UpdateBehavior == StackUpdateBehavior.Notify;
            var shouldAutoDeploy = gitStack.UpdateBehavior is StackUpdateBehavior.StackAutoDeploy or StackUpdateBehavior.ServiceAutoDeploy;
            ActivityEvent? availableActivity = null;

            if (shouldNotify)
            {
                availableActivity = CreateGitStackActivity(
                    stack,
                    currentCommit,
                    remoteCommit,
                    ActivityEventType.StackGitUpdateAvailable,
                    ActivityStatus.Warning,
                    new StackGitUpdateAvailable(repo.Name, branch, currentCommit, remoteCommit));

                await uow.ActivityEventRepository.AddAsync(availableActivity, cancellationToken);
                stack.AssignActivityEvent(availableActivity);
            }

            updates.Add(new LinkedGitStackUpdate(
                stack,
                repo.Name,
                branch,
                currentCommit,
                remoteCommit,
                shouldNotify,
                shouldAutoDeploy,
                availableActivity));
        }

        return updates;
    }

    private async Task<(bool Success, string? Error)> TryAutoDeployAsync(Guid stackId, CancellationToken cancellationToken)
    {
        string? error = null;

        try
        {
            await foreach (var item in applyStackService.ApplyAsync(
                stackId,
                Constants.SystemId,
                serviceNames: null,
                pullImages: true,
                cancellationToken))
            {
                if (!string.IsNullOrWhiteSpace(item.Message))
                {
                    error = item.Message;
                    break;
                }

            }
        }
        catch (Exception ex)
        {
            error = ex.Message;
        }

        return string.IsNullOrWhiteSpace(error)
            ? (true, null)
            : (false, error);
    }

    private Task ProcessGitStackAlertAsync(
        AlertType type,
        LinkedGitStackUpdate update,
        bool failed,
        string? reason,
        CancellationToken cancellationToken)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            StackGitUpdates:
            [
                new StackGitUpdateAlertSnapshot(
                    update.Stack.Id,
                    update.Stack.Name,
                    update.GitRepositoryName,
                    update.Branch,
                    update.CurrentCommitSha,
                    update.RemoteCommitSha,
                    failed,
                    reason)
            ]);

        return alertService.ProcessAsync(type, context, cancellationToken);
    }

    private ActivityEvent CreateGitStackActivity(
        LinkedGitStackUpdate update,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info)
        => CreateGitStackActivity(update.Stack, update.CurrentCommitSha, update.RemoteCommitSha, eventType, status, info);

    private static ActivityEvent CreateGitStackActivity(
        Stack stack,
        string currentCommitSha,
        string remoteCommitSha,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info)
        => new(
            actorId: Constants.SystemId,
            resourceId: stack.Id,
            platformId: stack.CurrentStackRelease?.PlatformId,
            resourceName: stack.Name,
            eventType: eventType,
            status: status,
            info: info);
}

internal sealed record LinkedGitStackUpdate(
    Stack Stack,
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha,
    bool ShouldNotify,
    bool ShouldAutoDeploy,
    ActivityEvent? AvailableActivity);

internal sealed class GitRepoSyncFailedWorkItem(
    IGitRepositoryStreamManager streamManager,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    GitOperation gitOperation,
    GitRepository repo,
    string branch,
    string errorMessage,
    GitRepoSyncTrigger trigger,
    bool BranchScopedFailure
    ) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var branchOnlyFailure = BranchScopedFailure
            && !GitRepoSyncScope.IsPrimaryRepositorySync(repo, branch, trigger);

        if (branchOnlyFailure)
        {
            await uow.GitRepositories.UpsertRefAsync(
                new GitRepositoryRef(repo.Id, branch, string.Empty, GitReposStatus.Degraded, errorMessage),
                cancellationToken);

            await uow.CommitAsync(cancellationToken);
            return;
        }

        // Update repo status
        repo.ReleaseProcessing(GitReposStatus.Degraded);
        await uow.GitRepositories.UpdateAsync(repo, cancellationToken);
        await uow.GitRepositories.UpsertRefAsync(
            new GitRepositoryRef(repo.Id, branch, string.Empty, GitReposStatus.Degraded, errorMessage),
            cancellationToken);

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
        repo.AssignActivityEvent(activity);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, cancellationToken)), cancellationToken);
        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(streamManager, repo), cancellationToken);
    }
}

public class GitRepoNotificationWorkItem(IGitRepositoryStreamManager gitRepoHub, GitRepository repo, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => gitRepoHub.SendGitRepoInfo(repo, action);
}

public enum GitRepoSyncTrigger
{
    Manual = 0,
    Poll = 1,
    Apply = 2,
    Webhook = 3
}

public readonly record struct GitRepoSyncRequest(
    Guid RepoId,
    string? Branch = null,
    GitRepoSyncTrigger Trigger = GitRepoSyncTrigger.Manual);

internal static class GitRepoSyncScope
{
    public static bool IsPrimaryRepositorySync(GitRepository repo, string branch, GitRepoSyncTrigger trigger)
        => trigger != GitRepoSyncTrigger.Poll
            || string.Equals(branch, repo.DefaultBranch, StringComparison.OrdinalIgnoreCase);
}
