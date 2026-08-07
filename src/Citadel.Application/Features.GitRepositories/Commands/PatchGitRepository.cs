using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Contracts.Resources;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using System.Threading.Channels;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Write)]
public sealed record PatchGitRepository(Guid Id, JsonMergePatchDocument<GitRepository> Patch) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : PatchCommandValidator<PatchGitRepository, GitRepository>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: GitJsonContext.Default.GitRepository,
                modelValidator: new GitRepositoryValidator())
        {
        }
    }

    internal sealed class GitRepositoryValidator : AbstractValidator<GitRepository>
    {
        public GitRepositoryValidator()
        {
            RuleFor(x => x.Id).NotEmpty();
            When(s => s.Url != null, () => RuleFor(x => x.Url).NotEmpty());
            When(x => x.SyncMode == GitRepositorySyncMode.PullInterval, () =>
                RuleFor(x => x.SyncIntervalMinutes).NotNull().GreaterThanOrEqualTo(1));
            RuleFor(x => x.Webhook)
                .Must(webhook => WebhookConfigurationValidation.GetAuthenticationError(webhook) is null)
                .WithMessage(repository => WebhookConfigurationValidation.GetAuthenticationError(repository.Webhook));
            RuleFor(x => x.Webhook!.Secret).MaximumLength(256).When(x => x.Webhook is not null);
            RuleFor(x => x.Webhook!.BranchFilter).MaximumLength(256).When(x => x.Webhook is not null);
        }
    }
}

internal sealed class PatchGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    IRepoCacheManager repoCacheManager,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager gitRepositoryHub,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    IUserContextAccessor userContext) : ICommandHandler<PatchGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(PatchGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken);
        if (gitRepository is null)
            return Result.Failure<GitRepository>(new NotFoundError("The provided git repository does not exist"));

        var patchedGitRepository = command.Patch.ApplyTo(gitRepository, GitJsonContext.Default.GitRepository);

        var validation = await GitRepositoryUrlValidation.ValidateAsync(unitOfWork, patchedGitRepository.GitAccountId, patchedGitRepository.Url, cancellationToken);
        if (validation.IsFailure())
            return Result.Failure<GitRepository>(validation.Errors);

        if (patchedGitRepository.GitAccountId != null && patchedGitRepository.GitAccountId != Guid.Empty)
        {
            var exists = await unitOfWork.GitAccounts.ExistsAsync(patchedGitRepository.GitAccountId.Value, cancellationToken);
            if (!exists)
                return Result.Failure<GitRepository>(new NotFoundError("The provided git account does not exist"));
        }

        var sourceChanged = !string.Equals(gitRepository.Url, patchedGitRepository.Url, StringComparison.OrdinalIgnoreCase)
            || !string.Equals(gitRepository.DefaultBranch, patchedGitRepository.DefaultBranch, StringComparison.OrdinalIgnoreCase)
            || gitRepository.GitAccountId != patchedGitRepository.GitAccountId;

        var previousCachePath =
            ApplicationStoragePaths.GetRepositoryCachePath(gitRepository);

        // Add Activity
        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: gitRepository.Id,
            platformId: null,
            resourceName: gitRepository.Name,
            eventType: ActivityEventType.GitRepoUpdated,
            status: ActivityStatus.Success,
            info: new GitRepoUpdated(gitRepository.ToSnapshot(), patchedGitRepository.ToSnapshot(command.Id)));

        gitRepository.PartialUpdate(
            defaultBranch: patchedGitRepository.DefaultBranch!,
            status: sourceChanged ? GitReposStatus.Pending : gitRepository.Status,
            webhook: patchedGitRepository.Webhook,
            onClone: patchedGitRepository.OnClone,
            onPull: patchedGitRepository.OnPull,
            syncMode: patchedGitRepository.SyncMode,
            syncIntervalMinutes: patchedGitRepository.SyncIntervalMinutes);

        gitRepository.UpdateSource(patchedGitRepository.Url, patchedGitRepository.GitAccountId);

        if (sourceChanged)
        {
            await repoCacheManager.DeleteCacheAsync(previousCachePath, cancellationToken);
        }

        if (sourceChanged)
        {
            gitRepository.MarkProcessing(actorId);
        }

        await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepositoryNotificationWorkItem(gitRepositoryHub, gitRepository), cancellationToken);

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        if (sourceChanged)
        {
            await gitSyncWriter.WriteAsync(new GitRepoSyncRequest(gitRepository.Id), CancellationToken.None);
        }

        return gitRepository;
    }

    internal sealed class GitRepositoryNotificationWorkItem(IGitRepositoryStreamManager gitRepositoryHub, GitRepository repository, string action = "update") : INotificationWorkItem
    {
        public Task ExecuteAsync(CancellationToken cancellationToken)
            => gitRepositoryHub.SendGitRepoInfo(repository, action);
    }
}
