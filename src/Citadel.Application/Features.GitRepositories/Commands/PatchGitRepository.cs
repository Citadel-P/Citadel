using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
using System.Threading.Channels;

namespace Application.Features.GitRepositories.Commands;

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
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<PatchGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(PatchGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

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


        gitRepository.PartialUpdate(
            defaultBranch: patchedGitRepository.DefaultBranch!,
            status: sourceChanged ? GitReposStatus.Pending : gitRepository.Status,
            webHookEnabled: patchedGitRepository.WebHookEnabled,
            webHookSecret: patchedGitRepository.WebHookSecret,
            onClone: patchedGitRepository.OnClone,
            onPull: patchedGitRepository.OnPull);

        gitRepository.UpdateSource(patchedGitRepository.Url, patchedGitRepository.GitAccountId);

        // Activity
        ActivityEvent? activity = activity = new ActivityEvent(
            actorId: actorId,
            resourceId: gitRepository.Id,
            platformId: null,
            resourceName: gitRepository.Name,
            eventType: ActivityEventType.GitRepoUpdated,
            status: ActivityStatus.Success,
            info: new GitRepoUpdated(gitRepository.ToSnapshot(), patchedGitRepository.ToSnapshot(command.Id)));
        
        if (sourceChanged && !string.Equals(gitRepository.GetCachePath(), gitRepository.GetCachePath(), StringComparison.OrdinalIgnoreCase))
        {
            await repoCacheManager.DeleteCacheAsync(gitRepository.GetCachePath(), cancellationToken);
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
