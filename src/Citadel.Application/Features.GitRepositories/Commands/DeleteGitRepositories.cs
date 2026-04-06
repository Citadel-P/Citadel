using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Claims;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, ResourceAction.Delete)]
public sealed record DeleteGitRepositories(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteGitRepositoriesHandler(
    IUnitOfWork unitOfWork,
    IServiceScopeFactory scopeFactory,
    IRepoCacheManager repoCacheManager,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager gitRepositoryHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<DeleteGitRepositories, Result>
{
    public async ValueTask<Result> Handle(DeleteGitRepositories command, CancellationToken cancellationToken)
    {
        var toDelete = await unitOfWork.GitRepositories.GetAllAsync(command.Ids, cancellationToken);
        if (toDelete is null || !toDelete.Any())
            return Result.Failure(new NotFoundError("No git repositories found matching the provided IDs for deletion."));

        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        foreach (var gitRepository in toDelete)
        {
            gitRepository.MarkProcessing(actorId);
            await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        foreach (var gitRepository in toDelete)
        {
            await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(gitRepositoryHub, gitRepository), cancellationToken);
        }

        foreach (var gitRepository in toDelete)
        {
            await repoCacheManager.DeleteCacheAsync(gitRepository, cancellationToken);
        }

        var result = await DeleteAsync(toDelete, actorId, cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No git repositories found matching the provided IDs for deletion."));
    }

    private async Task<int> DeleteAsync(IEnumerable<GitRepository> repositories, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

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
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct)), ct);
        }

        var deleted = await uow.GitRepositories.RemoveRangeAsync(repositories.Select(x => x.Id), ct);
        await uow.CommitAsync(ct);

        foreach (var repository in repositories)
        {
            await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(gitRepositoryHub, repository, "delete"), ct);
        }

        return deleted;
    }
}
