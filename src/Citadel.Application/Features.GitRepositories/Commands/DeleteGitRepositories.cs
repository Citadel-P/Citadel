using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Execute)]
public sealed record DeleteGitRepositories(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteGitRepositoriesHandler(
    IUnitOfWork unitOfWork,
    IServiceScopeFactory scopeFactory,
    IRepoCacheManager repoCacheManager,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager gitRepositoryHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeleteGitRepositoriesHandler> logger) : ICommandHandler<DeleteGitRepositories, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(DeleteGitRepositories command, CancellationToken cancellationToken)
    {
        var requestedIds = command.Ids.Distinct().ToArray();
        var toDelete = (await unitOfWork.GitRepositories.GetAllAsync(requestedIds, cancellationToken) ?? [])
            .ToArray();
        if (requestedIds.Length == 0 || toDelete.Length != requestedIds.Length)
            return Result.Failure(new NotFoundError("One or more git repositories were not found."));

        var actorId = userContext.Current.ActorId;
        var previousStatuses = toDelete.ToDictionary(repository => repository.Id, repository => repository.Status);
        foreach (var gitRepository in toDelete)
        {
            gitRepository.MarkProcessing(actorId);
            var claimed = await unitOfWork.GitRepositories.UpdateProcessingAsync(
                gitRepository.Id,
                gitRepository.Status,
                gitRepository.ControlState,
                gitRepository.ControlStartedAt,
                gitRepository.RowVersion,
                checkRowVersion: true,
                actorId,
                cancellationToken);
            if (claimed == 0)
            {
                await unitOfWork.RollbackAsync();
                RestoreClaimedEntities(toDelete, previousStatuses);
                return Result.Failure(new ConflictError(
                    "One or more git repositories changed while deletion was being claimed."));
            }
        }

        await unitOfWork.CommitAsync(cancellationToken);

        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;
        var deletionCommitted = false;

        try
        {
            foreach (var gitRepository in toDelete)
            {
                await TryNotifyAsync(
                    new GitRepoNotificationWorkItem(gitRepositoryHub, gitRepository),
                    completionToken);
            }

            var result = await DeleteAsync(toDelete, actorId, completionToken);
            if (result != toDelete.Length)
            {
                await TryRollbackClaimsAsync(toDelete, previousStatuses);
                return Result.Failure(new ConflictError(
                    "The git repository set changed while deletion was in progress."));
            }

            deletionCommitted = true;
            foreach (var gitRepository in toDelete)
            {
                await TryPostCommitStepAsync(
                    token => repoCacheManager.DeleteCacheAsync(gitRepository, token),
                    gitRepository.Id,
                    "repository cache cleanup",
                    completionToken);
            }

            return Result.Success();
        }
        catch
        {
            if (!deletionCommitted)
                await TryRollbackClaimsAsync(toDelete, previousStatuses);
            throw;
        }
    }

    private async Task<int> DeleteAsync(IEnumerable<GitRepository> repositories, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activityNotifications = new List<INotificationWorkItem>();

        foreach (var repository in repositories)
        {
            var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: repository.Id,
                platformId: null,
                resourceName: repository.Name,
                status: Domain.ActivityStatus.Success,
                eventType: Domain.ActivityEventType.GitRepoDeleted,
                info: new GitRepoDeleted(repository.ToSnapshot()));

            await uow.ActivityEventRepository.AddAsync(activity, ct);
            activityNotifications.Add(new ActivityNotificationWorkItem(
                activityHub,
                await activity.AssignActor(uow, ct)));
        }

        var deleted = await uow.GitRepositories.RemoveRangeAsync(repositories.Select(x => x.Id), ct);
        if (deleted != activityNotifications.Count)
        {
            await uow.RollbackAsync();
            return 0;
        }

        await uow.CommitAsync(ct);

        foreach (var notification in activityNotifications)
            await TryNotifyAsync(notification, ct);

        foreach (var repository in repositories)
        {
            await TryNotifyAsync(
                new GitRepoNotificationWorkItem(gitRepositoryHub, repository, "delete"),
                ct);
        }

        return deleted;
    }

    private async Task TryNotifyAsync(
        INotificationWorkItem notification,
        CancellationToken cancellationToken)
    {
        await TryPostCommitStepAsync(
            async token => await notificationQueue.EnqueueAsync(notification, token),
            null,
            "deletion notification",
            cancellationToken);
    }

    private async Task TryRollbackClaimsAsync(
        IReadOnlyCollection<GitRepository> repositories,
        IReadOnlyDictionary<Guid, GitReposStatus> previousStatuses)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            foreach (var repository in repositories)
            {
                var previousStatus = previousStatuses[repository.Id];
                repository.ReleaseProcessing(previousStatus);
                var restored = await unitOfWork.GitRepositories.UpdateProcessingAsync(
                    repository.Id,
                    previousStatus,
                    repository.ControlState,
                    repository.ControlStartedAt,
                    repository.RowVersion + 1,
                    checkRowVersion: true,
                    controlTriggeredBy: null,
                    rollbackCancellation.Token);
                if (restored == 0)
                {
                    await unitOfWork.RollbackAsync();
                    return;
                }
            }

            await unitOfWork.CommitAsync(rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back Git repository deletion claims");
        }
    }

    private static void RestoreClaimedEntities(
        IEnumerable<GitRepository> repositories,
        IReadOnlyDictionary<Guid, GitReposStatus> previousStatuses)
    {
        foreach (var repository in repositories)
            repository.ReleaseProcessing(previousStatuses[repository.Id]);
    }

    private async Task TryPostCommitStepAsync(
        Func<CancellationToken, Task> action,
        Guid? repositoryId,
        string step,
        CancellationToken cancellationToken)
    {
        using var stepCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            cancellationToken);
        stepCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        try
        {
            await action(stepCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogWarning(
                ex,
                "Failed {Step} after Git repository {RepositoryId} state committed",
                step,
                repositoryId);
        }
    }
}
