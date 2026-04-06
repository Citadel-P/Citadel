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
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
using System.Threading.Channels;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, ResourceAction.Create)]
public sealed record CreateGitRepository(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    bool WebHookEnabled = false,
    string? WebHookSecret = null,
    RepoCommand? OnClone = null,
    RepoCommand? OnPull = null) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : AbstractValidator<CreateGitRepository>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Url).NotEmpty();
            RuleFor(x => x.DefaultBranch).NotEmpty();
        }
    }
}

internal sealed class CreateGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager streamManager,
    ChannelWriter<GitRepoSyncRequest> gitSyncWriter,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(CreateGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

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

