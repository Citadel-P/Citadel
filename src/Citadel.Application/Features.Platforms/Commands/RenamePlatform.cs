using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record RenamePlatform(Guid Id, string Name) : ICommand<Result<Platform>>
{
    internal sealed class Validator : AbstractValidator<RenamePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenamePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformHub,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext) : ICommandHandler<RenamePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(RenamePlatform command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(command.Id, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Platform>(new NotFoundError("The provided platform does not exist"));
        }

        var conflict = await unitOfWork.Platforms.PlatformNameExistsAsync(command.Name, command.Id, cancellationToken);
        if (conflict != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same name already exists."));
        }

        var oldName = platform.Name;
        platform.PartialUpdate(name: command.Name);

        var activity = new ActivityEvent(
            actorId: userContext.Current.ActorId,
            resourceId: platform.Id,
            platformId: platform.Id,
            resourceName: command.Name,
            eventType: ActivityEventType.PlatformRenamed,
            status: ActivityStatus.Success,
            info: new PlatformRenamed(oldName, command.Name));

        await unitOfWork.Platforms.UpdateAsync(platform, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new PushPlatformUpdateNotificationWorkItem(platformHub, platform), cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return platform;
    }
}
