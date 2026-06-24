using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Threading.Channels;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Write)]
public sealed record CreateGitRepository(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    bool WebHookEnabled = false,
    string? WebHookSecret = null,
    RepoCommand? OnClone = null,
    RepoCommand? OnPull = null,
    GitRepositorySyncMode SyncMode = GitRepositorySyncMode.PullInterval,
    int? SyncIntervalMinutes = 5) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : AbstractValidator<CreateGitRepository>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Url).NotEmpty();
            RuleFor(x => x.DefaultBranch).NotEmpty();
            When(x => x.SyncMode == GitRepositorySyncMode.PullInterval, () =>
                RuleFor(x => x.SyncIntervalMinutes).NotNull().GreaterThanOrEqualTo(1));
        }
    }
}

internal sealed class CreateGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager streamManager,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    IUserContextAccessor userContext) : ICommandHandler<CreateGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(CreateGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var exists = await unitOfWork.GitRepositories.ExistsAsync(command.Name, cancellationToken);
        if (exists)
            return Result.Failure<GitRepository>(new ConflictError("Name already exists"));

        if (command.GitAccountId != null && command.GitAccountId != Guid.Empty)
        {
            var exist = await unitOfWork.GitAccounts.ExistsAsync(command.GitAccountId.Value, cancellationToken);
            if (exist == false)
                return Result.Failure<GitRepository>(new NotFoundError("Git account not found"));
        }

        var validation = await GitRepositoryUrlValidation.ValidateAsync(unitOfWork, command.GitAccountId, command.Url, cancellationToken);
        if (validation.IsFailure())
            return Result.Failure<GitRepository>(validation.Errors);

        // Add repository
        var gitRepository = new GitRepository(
            command.Name,
            command.Description,
            command.Url,
            command.DefaultBranch,
            command.GitAccountId,
            actorId,
            command.WebHookEnabled,
            command.WebHookSecret ?? string.Empty,
            command.OnClone,
            command.OnPull);

        gitRepository.UpdateSyncPolicy(command.SyncMode, command.SyncIntervalMinutes);

        gitRepository.MarkProcessing(actorId);

        await unitOfWork.GitRepositories.AddAsync(gitRepository, cancellationToken);

        // Add activity
        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: gitRepository.Id,
            platformId: null,
            resourceName: gitRepository.Name,
            eventType: ActivityEventType.GitRepoCreated,
            status: ActivityStatus.Information,
            info: new GitRepoCreated(gitRepository.ToSnapshot())
            );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        await notificationQueue.EnqueueAsync(new GitRepoNotificationWorkItem(streamManager, gitRepository, "create"), cancellationToken);

        // Clone the repo.
        await gitSyncWriter.WriteAsync(new GitRepoSyncRequest(gitRepository.Id), CancellationToken.None);

        return gitRepository;
    }
}

