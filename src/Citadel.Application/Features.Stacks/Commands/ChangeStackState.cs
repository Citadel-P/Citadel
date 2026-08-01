using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ChangeStackState(IEnumerable<Guid> Ids, StackAction Action) : ICommand<Result>;

public enum StackAction
{
    START = 0,
    STOP,
    PAUSE,
    UNPAUSE,
    RESTART
}

internal sealed class ChangeStackStateHandler(
    IServiceScopeFactory scopeFactory,
    IUserContextAccessor userContext,
    INotificationQueue notificationQueue,
    IStackStreamManager stackHub,
    IPlatformContainerCache platformCache,
    IContainerProcessingService containerProcessingService,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IHostApplicationLifetime applicationLifetime,
    ILogger<ChangeStackStateHandler> logger) : ICommandHandler<ChangeStackState, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(ChangeStackState command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var processedStacks = await MarkProcessingAsync(command.Ids, actorId, cancellationToken);

        if (processedStacks.Count == 0)
        {
            return Result.Failure(new NotFoundError("No stacks found for the provided stack ID(s)."));
        }

        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;
        try
        {
            var containers = await GetContainersAsync(processedStacks, completionToken);
            if (containers.Length == 0)
            {
                await TryRollbackProcessingAsync(processedStacks);
                return Result.Failure(new NotFoundError("No containers found for the provided stack ID(s)."));
            }

            var eligibleContainers = containers
                .Where(container => CanApply(command.Action, container.State))
                .ToArray();

            if (eligibleContainers.Length == 0)
            {
                await TryRollbackProcessingAsync(processedStacks);
                return Result.Failure(new BadRequestError(GetNoEligibleContainersMessage(command.Action)));
            }

            if (!CanComplete(command.Action, containers))
            {
                await TryRollbackProcessingAsync(processedStacks);
                return Result.Failure(new BadRequestError(GetUnsupportedStateCombinationMessage(command.Action)));
            }

            if (!platformCache.TryGetPlatformsWithContainers([.. eligibleContainers.Select(container => container.ContainerId)], out var platformContainers))
            {
                await TryRollbackProcessingAsync(processedStacks);
                return Result.Failure(new NotFoundError("Platform resolution failed for stack container ID(s). Platform may be disconnected."));
            }

            await NotifyProcessingAsync(processedStacks, completionToken);

            var containerAction = ActionMap.GetValueOrDefault(command.Action, ContainerAction.START);
            foreach (var platform in platformContainers)
            {
                var result = await PatchPlatformAsync(platform, containerAction, completionToken);

                if (result.IsFailure())
                {
                    await TryRollbackProcessingAsync(processedStacks);
                    return result;
                }
            }

            await containerProcessingService.CompleteProcessingAsync(
                new ProcessedResources([], [], processedStacks.Select(processed => processed.Stack).ToList()),
                platformContainers,
                actorId);
            return Result.Success();
        }
        catch
        {
            await TryRollbackProcessingAsync(processedStacks);
            throw;
        }
    }

    private async Task TryRollbackProcessingAsync(IReadOnlyCollection<ProcessedStack> processedStacks)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await RollbackProcessingAsync(processedStacks, rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back stack command claims");
        }
    }

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction containerAction, CancellationToken ct)
    {
        var cmd = new PatchContainerCommand(
            Action: containerAction,
            PlatformAddress: platform.Address,
            ContainerIds: platform.Containers.Select(s => s.Key));

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return connector.PatchAsync(cmd, ct);
    }

    private async Task<List<ProcessedStack>> MarkProcessingAsync(IEnumerable<Guid> stackIds, Guid actorId, CancellationToken ct)
    {
        var successfullyUpdated = new List<ProcessedStack>();
        var requestedIds = stackIds.Distinct().Order().ToArray();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var stacks = new List<Stack>(requestedIds.Length);
        foreach (var stackId in requestedIds)
        {
            var stack = await uow.Stacks.GetAsync(stackId, ct);
            if (stack?.CurrentStackRelease is null || stack.ControlState == ResourceControlState.Processing)
                return [];

            stacks.Add(stack);
        }

        foreach (var stack in stacks)
        {
            var previousStatus = stack.CurrentStackRelease!.Status;
            if (!stack.MarkProcessing(actorId))
                return [];

            var affectedRow = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                stack.CurrentStackRelease.Status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion,
                checkRowVersion: true,
                actorId,
                ct);

            if (!affectedRow)
                return [];

            successfullyUpdated.Add(new ProcessedStack(stack, previousStatus));
        }

        await uow.CommitAsync(ct);
        return successfullyUpdated;
    }

    private async Task<StackContainerTarget[]> GetContainersAsync(IEnumerable<ProcessedStack> processedStacks, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = new Dictionary<string, StackContainerTarget>(StringComparer.OrdinalIgnoreCase);

        foreach (var processedStack in processedStacks)
        {
            var stackContainers = await uow.Stacks.GetContainersAsync(processedStack.Stack.Id, ct);

            foreach (var container in stackContainers)
            {
                if (!string.IsNullOrWhiteSpace(container.DockerContainerId))
                {
                    containers.TryAdd(
                        container.DockerContainerId,
                        new StackContainerTarget(container.DockerContainerId, container.State));
                }
            }
        }

        return [.. containers.Values];
    }

    private static bool CanApply(StackAction action, ContainerStateStatus state)
    {
        return action switch
        {
            StackAction.START => state is ContainerStateStatus.Created or ContainerStateStatus.Exited,
            StackAction.STOP => state is ContainerStateStatus.Running or ContainerStateStatus.Paused or ContainerStateStatus.Restarting,
            StackAction.PAUSE => state is ContainerStateStatus.Running,
            StackAction.UNPAUSE => state is ContainerStateStatus.Paused,
            StackAction.RESTART => state is ContainerStateStatus.Running or ContainerStateStatus.Restarting,
            _ => false
        };
    }

    private static bool CanComplete(StackAction action, IEnumerable<StackContainerTarget> containers)
    {
        return action switch
        {
            StackAction.START => containers.All(container =>
                container.State is ContainerStateStatus.Created
                    or ContainerStateStatus.Exited
                    or ContainerStateStatus.Running),
            StackAction.STOP => containers.All(container =>
                container.State is ContainerStateStatus.Exited
                    or ContainerStateStatus.Offline
                    or ContainerStateStatus.Paused
                    or ContainerStateStatus.Restarting
                    or ContainerStateStatus.Running),
            StackAction.PAUSE or StackAction.UNPAUSE => containers.All(container =>
                container.State is ContainerStateStatus.Paused
                    or ContainerStateStatus.Running),
            StackAction.RESTART => containers.All(container =>
                container.State is ContainerStateStatus.Running
                    or ContainerStateStatus.Restarting),
            _ => false
        };
    }

    private static string GetNoEligibleContainersMessage(StackAction action)
    {
        var actionLabel = action.ToString().ToLowerInvariant();
        return $"No stack containers are currently eligible to {actionLabel}.";
    }

    private static string GetUnsupportedStateCombinationMessage(StackAction action)
    {
        var actionLabel = action.ToString().ToLowerInvariant();
        return $"The current stack container states cannot complete a stable {actionLabel} operation.";
    }

    private async Task RollbackProcessingAsync(IEnumerable<ProcessedStack> processedStacks, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var released = new List<ProcessedStack>();

        foreach (var processedStack in processedStacks)
        {
            processedStack.Stack.ReleaseProcessing(processedStack.PreviousStatus);

            var affected = await uow.Stacks.UpdateProcessingAsync(
                processedStack.Stack.Id,
                processedStack.PreviousStatus,
                processedStack.Stack.ControlState,
                processedStack.Stack.ControlStartedAt,
                processedStack.Stack.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);
            if (affected)
                released.Add(processedStack);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(released, ct);
    }

    private async Task NotifyProcessingAsync(IEnumerable<ProcessedStack> processedStacks, CancellationToken ct)
    {
        using var notificationCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            ct);
        notificationCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        try
        {
            foreach (var processedStack in processedStacks)
            {
                await notificationQueue.EnqueueAsync(
                    new StackNotificationWorkItem(stackHub, processedStack.Stack),
                    notificationCancellation.Token);
            }
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to enqueue stack processing notifications");
        }
    }

    private sealed record ProcessedStack(Stack Stack, StackReleaseStatus PreviousStatus);

    private sealed record StackContainerTarget(string ContainerId, ContainerStateStatus State);

    private static readonly IReadOnlyDictionary<StackAction, ContainerAction> ActionMap =
        new Dictionary<StackAction, ContainerAction>
        {
            [StackAction.START] = ContainerAction.START,
            [StackAction.STOP] = ContainerAction.STOP,
            [StackAction.PAUSE] = ContainerAction.PAUSE,
            [StackAction.UNPAUSE] = ContainerAction.UNPAUSE,
            [StackAction.RESTART] = ContainerAction.RESTART
        };
}
