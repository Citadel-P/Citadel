using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Application.Features.GitRepositories.Commands.PatchGitRepositoryHandler;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Write)]
public sealed record RenameGitRepository(Guid Id, string Name) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : AbstractValidator<RenameGitRepository>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IGitRepositoryStreamManager gitRepositoryHub,
    IUserContextAccessor userContext) : ICommandHandler<RenameGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(RenameGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken);
        if (gitRepository is null)
            return Result.Failure<GitRepository>(new NotFoundError("The provided git repository does not exist"));

        var conflict = await unitOfWork.GitRepositories.ExistsAsync(command.Id, command.Name, cancellationToken);
        if (conflict)
            return Result.Failure<GitRepository>(new ConflictError("Name already exists"));

        gitRepository.PartialUpdate(name: command.Name);
        await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: gitRepository.Id,
            platformId: null,
            resourceName: command.Name,
            eventType: ActivityEventType.GitRepoRenamed,
            status: ActivityStatus.Success,
            info: new GitRepoRenamed(gitRepository.Name, command.Name));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepositoryNotificationWorkItem(gitRepositoryHub, gitRepository), cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return gitRepository;
    }

}
