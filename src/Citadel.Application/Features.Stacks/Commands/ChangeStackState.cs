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

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ChangeStackState(IEnumerable<Guid> Ids, StackAction Action) : ICommand<Result>;

public enum StackAction
{
    START = 0,
    STOP,
    PAUSE,
    UNPAUSE
}

internal sealed class ChangeStackStateHandler(
    IServiceScopeFactory scopeFactory,
    IUserContextAccessor userContext,
    INotificationQueue notificationQueue,
    IStackStreamManager stackHub,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IContainerConnector> connectorFactory) : ICommandHandler<ChangeStackState, Result>
{
    public async ValueTask<Result> Handle(ChangeStackState command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var processedStacks = await MarkProcessingAsync(command.Ids, actorId, cancellationToken);

        if (processedStacks.Count == 0)
        {
            return Result.Failure(new NotFoundError("No stacks found for the provided stack ID(s)."));
        }

        var containers = await GetContainersAsync(processedStacks, cancellationToken);
        if (containers.Length == 0)
        {
            await RollbackProcessingAsync(processedStacks, actorId, cancellationToken);
            return Result.Failure(new NotFoundError("No containers found for the provided stack ID(s)."));
        }

        var eligibleContainers = containers
            .Where(container => CanApply(command.Action, container.State))
            .ToArray();

        if (eligibleContainers.Length == 0)
        {
            await RollbackProcessingAsync(processedStacks, actorId, cancellationToken);
            return Result.Failure(new BadRequestError(GetNoEligibleContainersMessage(command.Action)));
        }

        if (!CanComplete(command.Action, containers))
        {
            await RollbackProcessingAsync(processedStacks, actorId, cancellationToken);
            return Result.Failure(new BadRequestError(GetUnsupportedStateCombinationMessage(command.Action)));
        }

        if (!platformCache.TryGetPlatformsWithContainers([.. eligibleContainers.Select(container => container.ContainerId)], out var platformContainers))
        {
            await RollbackProcessingAsync(processedStacks, actorId, cancellationToken);
            return Result.Failure(new NotFoundError("Platform resolution failed for stack container ID(s). Platform may be disconnected."));
        }

        await NotifyProcessingAsync(processedStacks, cancellationToken);

        var containerAction = ActionMap.GetValueOrDefault(command.Action, ContainerAction.START);
        foreach (var platform in platformContainers)
        {
            var result = await PatchPlatformAsync(platform, containerAction, cancellationToken);

            if (result.IsFailure())
            {
                await RollbackProcessingAsync(processedStacks, actorId, cancellationToken);
                return result;
            }
        }

        return Result.Success();
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

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var stackId in stackIds.Distinct())
        {
            var stack = await uow.Stacks.GetAsync(stackId, ct);
            if (stack?.CurrentStackRelease is null)
            {
                continue;
            }

            var previousStatus = stack.CurrentStackRelease.Status;
            if (!stack.MarkProcessing(actorId))
            {
                continue;
            }

            var affectedRow = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                stack.CurrentStackRelease.Status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion,
                checkRowVersion: true,
                actorId,
                ct);

            if (affectedRow)
            {
                successfullyUpdated.Add(new ProcessedStack(stack, previousStatus));
            }
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

    private async Task RollbackProcessingAsync(IEnumerable<ProcessedStack> processedStacks, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var processedStack in processedStacks)
        {
            processedStack.Stack.ReleaseProcessing(processedStack.PreviousStatus);

            await uow.Stacks.UpdateProcessingAsync(
                processedStack.Stack.Id,
                processedStack.PreviousStatus,
                processedStack.Stack.ControlState,
                processedStack.Stack.ControlStartedAt,
                processedStack.Stack.RowVersion,
                checkRowVersion: false,
                actorId,
                ct);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(processedStacks, ct);
    }

    private async Task NotifyProcessingAsync(IEnumerable<ProcessedStack> processedStacks, CancellationToken ct)
    {
        foreach (var processedStack in processedStacks)
        {
            await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, processedStack.Stack), ct);
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
            [StackAction.UNPAUSE] = ContainerAction.UNPAUSE
        };
}
