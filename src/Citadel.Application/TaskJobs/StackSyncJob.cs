using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Synchronizes stack release state from platform health and container state.
/// </summary>
internal sealed class StackSyncJob(
    IDbWorkQueue dbWorkQueue,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IStackStreamManager stackStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ILogger<StackSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{JobName} started. Running every {H} hours.", nameof(StackSyncJob), SyncInterval.TotalHours);

        var eventDrivenTask = RunEventDrivenSync(cancellationToken);
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    private async Task RunEventDrivenSync(CancellationToken ct)
    {
        await foreach (var platformEvent in platformHealthReader.ReadAllAsync(ct))
        {
            try
            {
                if (platformEvent.IsOnLine && !platformEvent.IsValidated)
                    continue;

                await ScheduleStackSync(platformEvent.Id, platformEvent.IsOnLine, ct);
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Unhandled error while scheduling stack sync for platform {PlatformId}",
                    platformEvent.Id);
            }
        }
    }

    private async Task RunPeriodicSync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                if (platformContainerCache.TryGetCacheEntries(out var platforms, out _))
                {
                    foreach (var platform in platforms)
                    {
                        await ScheduleStackSync(platform.Id, true, ct);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "periodic stack sync failed");
            }

            await Task.Delay(SyncInterval, ct);
        }
    }

    private ValueTask ScheduleStackSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(new StackSyncWorkItem(stackStreamManager, notificationQueue, platformId, isOnline), ct);
}

internal sealed class StackSyncWorkItem(
    IStackStreamManager stackStreamManager,
    INotificationQueue notificationQueue,
    Guid platformId,
    bool platformIsOnline) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var updated = new List<Stack>();
        var stacks = (await uow.Stacks.GetInfoAsync(ct, platformId: platformId))
            .Where(static stack => stack.CurrentStackRelease?.Platform?.PlatformDescriptor.Type == PlatformType.Docker)
            .ToArray();

        if (stacks.Length == 0)
            return;

        var containersByStackId = platformIsOnline
            ? (await uow.Containers.GetByPlatformIdAsync(platformId, ct))
                .Where(container => container.StackId.HasValue)
                .GroupBy(container => container.StackId!.Value)
                .ToDictionary(group => group.Key, group => group.ToArray())
            : [];

        foreach (var stack in stacks)
        {
            var release = stack.CurrentStackRelease;
            if (release is null || release.Status == StackReleaseStatus.Created)
                continue;

            if (stack.ControlState == ResourceControlState.Processing)
            {
                if (release.Status is StackReleaseStatus.Applying or StackReleaseStatus.Pending)
                    continue;

                stack.ReleaseProcessing(release.Status);
                await uow.Stacks.UpdateProcessingAsync(
                    id: stack.Id,
                    status: release.Status,
                    state: stack.ControlState,
                    startedAt: stack.ControlStartedAt,
                    rowVersion: stack.RowVersion,
                    checkRowVersion: false,
                    controlTriggeredBy: stack.ControlTriggeredBy,
                    cancellationToken: ct);
                updated.Add(stack);
                continue;
            }

            var nextStatus = platformIsOnline
                ? GetOnlineStatus(stack, containersByStackId)
                : StackReleaseStatus.Degraded;

            if (nextStatus == release.Status)
                continue;

            stack.PartialUpdate(nextStatus);
            await uow.Stacks.UpdateReleaseStatusAsync(release.Id, nextStatus, ct);
            updated.Add(stack);
        }

        await uow.CommitAsync(ct);

        foreach (var stack in updated)
        {
            await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackStreamManager, stack), ct);
        }
    }

    private static StackReleaseStatus GetOnlineStatus(
        Stack stack,
        IReadOnlyDictionary<Guid, Domain.Entities.Container[]> containersByStackId)
    {
        if (!containersByStackId.TryGetValue(stack.Id, out var containers) || containers.Length == 0)
            return StackReleaseStatus.Degraded;

        var states = containers.Select(container => container.State).ToArray();
        return states.Any(state => state == ContainerStateStatus.Offline)
            ? StackReleaseStatus.Degraded
            : Stack.ToStackStatus(states);
    }
}
