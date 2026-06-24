using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Threading.Channels;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Execute)]
public sealed record SyncGitRepository(Guid Id, string? Branch = null) : ICommand<Result<GitRepository>>;

internal sealed class SyncGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IGitRepositoryStreamManager gitRepositoryHub,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    IUserContextAccessor userContext) : ICommandHandler<SyncGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(SyncGitRepository command, CancellationToken cancellationToken)
    {
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken);
        if (gitRepository is null)
            return Result.Failure<GitRepository>(new NotFoundError("The provided git repository does not exist"));

        var actorId = userContext.Current.ActorId;
        gitRepository.MarkProcessing(actorId);

        await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(gitRepositoryHub, gitRepository), cancellationToken);

        await gitSyncWriter.WriteAsync(
            new GitRepoSyncRequest(command.Id, command.Branch, GitRepoSyncTrigger.Manual),
            CancellationToken.None);

        return gitRepository;
    }
}
