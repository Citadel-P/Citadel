using Application.Features.Deployments.Notifications;
using Application.Features.Platforms;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record DeletePlatforms(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeletePlatforms>
    {
        public Validator()
            => RuleForEach(s => s.Ids).NotNull();
    }
}

internal class DeletePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformStreamManager,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatforms, Result>
{
    public async ValueTask<Result> Handle(DeletePlatforms command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;

        foreach (var platformId in command.Ids)
        {
            var deploymentsExist = await unitOfWork.Deployments.ExistsAsync(platformId, cancellationToken);
            if (deploymentsExist)
            {
                return Result.Failure(new ConflictError("Platform has active deployment(s), delete or migrate them first."));
            }

            var stacksExist = await unitOfWork.Stacks.ExistsAsync(platformId, cancellationToken);
            if (stacksExist)
            {
                return Result.Failure(new ConflictError("Platform has active stack(s), delete or migrate them first."));
            }

            var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
            if (platform is null)
            {
                return Result.Failure(new NotFoundError("Platform does not exist"));
            }

            var activity = PlatformActivity.Deleted(platform, actorId);
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            await unitOfWork.Platforms.DeleteAsync(platformId, cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            await platformHealthMonitorJob.UntrackPlatform(platform.Address, cancellationToken);

            // Notify subscribers
            await platformStreamManager.PlatformDeleted(platformId);
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

            logger.LogInformation("Platform {Id} deleted successfully", platformId);
        }

        return Result.Success();
    }
}
