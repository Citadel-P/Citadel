using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;
using static Google.Rpc.Help.Types;

namespace Application.Services;

internal interface IApplyStackService
{
    IAsyncEnumerable<StackStreamItem> ApplyAsync(Guid stackId, CancellationToken ct);
}

internal class ApplyStackService(
    IDbWorkQueue dbWorkQueue,
    IStackStreamManager stackHub,
    IUserContextAccessor userContext,
    IServiceScopeFactory scopeFactory,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IStackConnector> stackConnectorFactory) : IApplyStackService
{
    public async IAsyncEnumerable<StackStreamItem> ApplyAsync(Guid stackId, [EnumeratorCancellation] CancellationToken ct)
    {
        var actorId = userContext.Current.ActorId;
        var stack = await LoadStack(stackId, ct);

        if (stack is null)
        {
            yield return StackStreamItem.FromStdErr($"❌ Stack with ID {stackId} not found.");
            yield break;
        }

        if (stack.CurrentStackRelease?.Spec is null)
        {
            var message = $"❌ Stack with ID {stackId} has no spec defined.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message);
            yield break;
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out _))
        {
            var message = "❌ Platform not found or disconnected.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message);
            yield break;
        }

        var currentRelease = stack.CurrentStackRelease;

        if (currentRelease.Spec is not ManualStack manualStack)
        {
            var message = "Only manual stack specs are currently supported for stack apply.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message);
            yield break;
        }

        if (string.IsNullOrWhiteSpace(manualStack.ComposeFile))
        {
            var message = "❌ Manual stack compose file is required.";
            await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
            yield return StackStreamItem.FromStdErr(message);
            yield break;
        }

        string? registryAuth = null;
        string? registryName = null;
        string? registryHost = null;
        if (currentRelease.Spec.RegistryId is Guid registryId && registryId != Guid.Empty)
        {
            var registry = await LoadRegistry(registryId, ct);
            if (registry is null)
            {
                var message = $"❌ Registry with ID {registryId} not found.";
                await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, message, ct: ct);
                yield return StackStreamItem.FromStdErr(message);
                yield break;
            }

            var host = registry.RegistryHost.Contains("://", StringComparison.Ordinal)
                ? registry.RegistryHost
                : $"https://{registry.RegistryHost}";

            registryHost = new Uri(host).Host.ToLowerInvariant();
            registryAuth = registry.Configuration.GetRegistryAuth(registryHost);
            registryName = registry.Name;
        }

        var markResult = await MarkProcessingAsync(stack.Id, actorId, ct);
        if (!markResult.IsSuccess)
        {
            var message = markResult.ErrorMessage ?? "❌ Stack is already being processed.";
            yield return StackStreamItem.FromStdErr(message);
            yield break;
        }

        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, markResult.Stack!), ct);

        yield return StackStreamItem.FromStdOut($"Applying stack to {platform.Address}...");

        var connector = stackConnectorFactory.GetConnector(platform.ConnectorType);
        var command = BuildApplyCommand(stack, platform.Address, manualStack, registryAuth, registryName, registryHost);

        int? exitCode = null;
        var errorLogs = new List<string>(); 
        var enumerator = connector.StackApplyAsync(command, ct).GetAsyncEnumerator(ct);

        try
        {
            while (true)
            {
                var next = await TryReadNextAsync(enumerator);
                if (next.ErrorMessage is not null)
                {
                    await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, next.ErrorMessage, ct: ct);
                    yield return StackStreamItem.FromStdErr(next.ErrorMessage);
                    yield break;
                }

                if (!next.HasItem || next.Result is null)
                {
                    break;
                }

                var result = next.Result;

                if (!string.IsNullOrWhiteSpace(result.Message))
                {
                    if (result.Type == StackApplyEventType.SystemMessage)
                    {
                        yield return StackStreamItem.SystemMessage(result.Message);
                    }
                    else
                    {
                        // Docker writes warnings and progress to stderr. 
                        // Only treat it as a critical failure message if it contains "error" or "failed"
                        if (result.Message.Contains("error", StringComparison.OrdinalIgnoreCase) ||
                            result.Message.Contains("failed", StringComparison.OrdinalIgnoreCase))
                        {
                            errorLogs.Add(result.Message);
                        }
                        else
                        {
                            yield return StackStreamItem.FromStdOut(result.Message);
                        }
                    }
                }

                if (result.ExitCode.HasValue)
                {
                    exitCode = result.ExitCode;
                    yield return StackStreamItem.Finished(result.ExitCode.Value);

                    if (result.ExitCode != 0)
                    {
                        var explicitFailure = errorLogs.Count > 0
                            ? string.Join(Environment.NewLine, errorLogs)
                            : $"❌ Pipeline command failed with exit code {result.ExitCode}.";

                        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, explicitFailure, ct: ct);
                        yield return StackStreamItem.FromStdErr($"❌ {explicitFailure}");
                        yield break;
                    }
                }
            }
        }
        finally
        {
            await enumerator.DisposeAsync();
        }

        if (exitCode == 0)
        {
            await dbWorkQueue.EnqueueAsync(
                new StackSucceededWorkItem(
                    stack.Id,
                    actorId,
                    stackHub,
                    activityHub,
                    notificationQueue),
                ct);

            yield return StackStreamItem.SystemMessage("✅ Stack apply completed successfully.");
            yield break;
        }

        var finalFailureMessage = errorLogs.Count > 0
            ? string.Join(Environment.NewLine, errorLogs)
            : (exitCode is int code ? $"❌ docker compose exited with code {code}." : "❌ Stack apply did not report a completion exit code.");

        await EnqueueStatus(stack.Id, actorId, StackReleaseStatus.Failed, finalFailureMessage, ct: ct);

        yield return StackStreamItem.FromStdErr(finalFailureMessage);
    }

    private static StackApplyCommand BuildApplyCommand(Stack stack, string platformAddress, ManualStack manualStack, 
        string? registryAuth, string? registryName, string? registryHost)
        => new(
            PlatformAddress: platformAddress,
            StackName: stack.Name,
            ComposeFileContent: manualStack.ComposeFile,
            ProjectName: manualStack.ProjectName ?? stack.Name,
            EnvironmentFilePath: manualStack.EnvFilePath,
            EnvironmentVariables: manualStack.EnvVars,
            PreDeploy: manualStack.PreDeploy,
            PostDeploy: manualStack.PostDeploy,
            RegistryAuth: registryAuth,
            RegistryName: registryName,
            RegistryHost: registryHost,
            DestroyBeforeDeploy: manualStack.DestroyBeforeDeploy,
            Spec: manualStack);


    private static async Task<(bool HasItem, StackApplyResult? Result, string? ErrorMessage)> TryReadNextAsync(IAsyncEnumerator<StackApplyResult> enumerator)
    {
        try
        {
            var hasItem = await enumerator.MoveNextAsync();
            return hasItem
                ? (true, enumerator.Current, null)
                : (false, null, null);
        }
        catch (Exception ex)
        {
            return (false, null, $"Stack apply failed: {ex.Message}");
        }
    }

    private async Task<(bool IsSuccess, Stack? Stack, string? ErrorMessage)> MarkProcessingAsync(Guid stackId, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null)
        {
            return (false, null, $"Stack with ID {stackId} not found.");
        }

        if (!stack.MarkProcessing(actorId))
        {
            return (false, null, "Stack is already being processed.");
        }

        stack.PartialUpdate(StackReleaseStatus.Applying);
        await uow.Stacks.UpdateAsync(stack, ct);
        await uow.CommitAsync(ct);
        return (true, stack, null);
    }

    private async Task<Stack?> LoadStack(Guid id, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Stacks.GetAsync(id, ct);
    }

    private async Task<Registry?> LoadRegistry(Guid registryId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Registries.GetAsync(registryId, ct);
    }

    private ValueTask EnqueueStatus(Guid stackId, Guid actorId, StackReleaseStatus status, string? message, CancellationToken ct = default)
        => dbWorkQueue.EnqueueAsync(
            new UpdateStackStatusWorkItem(
                stackId,
                actorId,
                status,
                message,
                stackHub,
                activityHub,
                notificationQueue),
            ct);
}

internal sealed class UpdateStackStatusWorkItem(Guid stackId, Guid actorId, StackReleaseStatus status, string? message, IStackStreamManager stackHub,
    IActivityStreamManager activityHub, INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null) return;

        stack.ReleaseProcessing(status);
        await uow.Stacks.UpdateAsync(stack, ct);

        // Add activity event
        ActivityEvent? activity = null;
        if (status == StackReleaseStatus.Failed)
        {
            activity = new ActivityEvent(
                            actorId: actorId,
                            resourceId: stack.Id,
                            platformId: stack.CurrentStackRelease?.PlatformId ?? Guid.Empty,
                            resourceName: stack.Name,
                            status: ActivityStatus.Failure,
                            eventType: ActivityEventType.StackApplied,
                            info: new StackApplied(stack.ToSnapshot(), new StackResultSnapshot(null, Helpers.RemoveAnsiSequences(message ?? "")))
                            );

            await uow.ActivityEventRepository.AddAsync(activity, ct);
        }

        await uow.CommitAsync(ct);

        // Push notifications

        var stackWorkItem = new StackNotificationWorkItem(stackHub, stack);
        await notificationQueue.EnqueueAsync(stackWorkItem, ct);

        if (activity is not null)
        {
            var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
            await notificationQueue.EnqueueAsync(activityWorkItem, ct);
        }
    }
}

internal sealed class StackSucceededWorkItem(
    Guid stackId,
    Guid actorId,
    IStackStreamManager stackHub,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var stack = await uow.Stacks.GetAsync(stackId, ct);
        if (stack is null) return;

        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        await uow.Stacks.UpdateAsync(stack, ct);

        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: stack.Id,
                        platformId: stack.CurrentStackRelease?.PlatformId,
                        resourceName: stack.Name,
                        status: ActivityStatus.Success,
                        eventType: ActivityEventType.StackApplied,
                        info: new StackApplied(stack.ToSnapshot(), new StackResultSnapshot(Message: "Stack applied successfully."))
                        );

        await uow.ActivityEventRepository.AddAsync(activity, ct);

        // Todo: attach containers
        await uow.CommitAsync(ct);

        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, stack), ct);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct)), ct);
    }
}

internal sealed class StackNotificationWorkItem(IStackStreamManager stackHub, Stack stack, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => stackHub.SendStackInfo(stack, action);
}
