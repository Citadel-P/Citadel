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
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record DeletePlatforms(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeletePlatforms>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal class DeletePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformStreamManager,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    ISyncBarrier syncBarrier,
    IPlatformContainerCache platformContainerCache,
    IUserContextAccessor userContext,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatforms, Result>
{
    public async ValueTask<Result> Handle(DeletePlatforms command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var platformIds = command.Ids.Distinct().ToArray();
        var deletions = new List<PlatformDeletion>(platformIds.Length);

        // Validate the complete batch before changing any state
        foreach (var platformId in platformIds)
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
            await activity.AssignActor(unitOfWork, cancellationToken);
            deletions.Add(new PlatformDeletion(platformId, platform.Address, activity));
        }

        foreach (var deletion in deletions)
        {
            var activity = deletion.Activity;
            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            var deleted = await unitOfWork.Platforms.DeleteAsync(deletion.PlatformId, cancellationToken);
            if (deleted == 0)
            {
                await unitOfWork.RollbackAsync();
                return Result.Failure(new ConflictError(
                    "The platform set changed while deletion was in progress."));
            }
        }

        await unitOfWork.CommitAsync(cancellationToken);

        // The database commit is authoritative. Do not let a disconnected HTTP client make a
        // committed delete look failed or prevent in-memory tracking from being cleaned up.
        using var cleanupCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        cleanupCancellation.CancelAfter(TimeSpan.FromSeconds(10));
        foreach (var deletion in deletions)
        {
            platformContainerCache.EvictPlatform(deletion.PlatformId);
            syncBarrier.RemovePlatform(deletion.PlatformId);

            await RunPostCommitStepAsync(
                () => platformHealthMonitorJob.UntrackPlatform(deletion.Address, cleanupCancellation.Token),
                deletion.PlatformId,
                "health monitor cleanup");
            await RunPostCommitStepAsync(
                () => platformStreamManager.PlatformDeleted(deletion.PlatformId).WaitAsync(cleanupCancellation.Token),
                deletion.PlatformId,
                "platform notification");
            await RunPostCommitStepAsync(
                async () => await notificationQueue.EnqueueAsync(
                    new ActivityNotificationWorkItem(activityHub, deletion.Activity),
                    cleanupCancellation.Token),
                deletion.PlatformId,
                "activity notification");

            logger.LogInformation("Platform {Id} deleted successfully", deletion.PlatformId);
        }

        return Result.Success();
    }

    private async Task RunPostCommitStepAsync(Func<Task> action, Guid platformId, string step)
    {
        try
        {
            await action();
        }
        catch (OperationCanceledException) when (applicationLifetime.ApplicationStopping.IsCancellationRequested)
        {
            logger.LogInformation(
                "Skipped {Step} for deleted platform {PlatformId} because the application is stopping",
                step,
                platformId);
        }
        catch (OperationCanceledException)
        {
            logger.LogWarning("Timed out during {Step} for deleted platform {PlatformId}", step, platformId);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed {Step} for deleted platform {PlatformId}", step, platformId);
        }
    }

    private sealed record PlatformDeletion(
        Guid PlatformId,
        string Address,
        Domain.Entities.Activities.ActivityEvent Activity);
}
