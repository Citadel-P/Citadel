using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Cryptography;
using System.Text;

namespace Application.Services;

public interface IStackDriftChecker
{
    Task<StackDriftReport> CheckAsync(Guid stackId, CancellationToken cancellationToken);
    Task<StackDriftReport> CheckAsync(StackDriftStack stack, CancellationToken cancellationToken);
}

public interface IStackReconciler
{
    Task<StackReconciliationResult> ReconcileAsync(Guid stackId, CancellationToken cancellationToken);
}

internal interface IStackDesiredStateProvider
{
    Task<StackDesiredState> GetDesiredStateAsync(StackDriftStack stack, CancellationToken cancellationToken);
}

internal interface IStackRuntimeStateProvider
{
    Task<StackRuntimeState> GetRuntimeStateAsync(StackDriftStack stack, CancellationToken cancellationToken);
}

internal sealed class StackDesiredStateProvider(IStackStoragePathProvider stackStoragePathProvider) : IStackDesiredStateProvider
{
    public Task<StackDesiredState> GetDesiredStateAsync(StackDriftStack stack, CancellationToken cancellationToken)
    {
        if (stack.Spec is ManualStack manualStack)
        {
            return Task.FromResult(Parse(stack, manualStack));
        }

        if (stack.Spec is GitStack gitStack)
        {
            return GetGitDesiredStateAsync(stack, gitStack, cancellationToken);
        }

        return Task.FromResult(new StackDesiredState(
            ProjectName: StackProjectNameResolver.Resolve(stack),
            Services: new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase)));
    }

    private async Task<StackDesiredState> GetGitDesiredStateAsync(
        StackDriftStack stack,
        GitStack gitStack,
        CancellationToken cancellationToken)
    {
        var sourceRoot = TryGetCurrentSourceRoot(stack.Id);
        var composePaths = stack.Source?.ComposePaths is { Count: > 0 }
            ? stack.Source.ComposePaths
            : gitStack.ComposePaths;

        if (sourceRoot is null || composePaths is not { Count: > 0 })
        {
            return new StackDesiredState(
                ProjectName: StackProjectNameResolver.Resolve(stack),
                Services: new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase));
        }

        var composeFiles = new List<string>(composePaths.Count);
        foreach (var composePath in composePaths)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var resolvedPath = ResolveRepositoryFile(sourceRoot, composePath);
            if (resolvedPath is null)
            {
                continue;
            }

            composeFiles.Add(await File.ReadAllTextAsync(resolvedPath, cancellationToken));
        }

        var services = StackComposeParser.ParseServices(stack.Id, stack.CurrentStackReleaseId, composeFiles)
            .ToDictionary(
                item => item.Key,
                item => new StackDesiredService(
                    item.Value.ServiceName,
                    item.Value.Image,
                    item.Value.ExpectedConfigHash),
                StringComparer.OrdinalIgnoreCase);

        return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
    }

    private static StackDesiredState Parse(StackDriftStack stack, ManualStack manualStack)
    {
        if (string.IsNullOrWhiteSpace(manualStack.ComposeFile))
        {
            return new StackDesiredState(
                StackProjectNameResolver.Resolve(stack),
                new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase));
        }

        var services = StackComposeParser.ParseServices(stack.Id, stack.CurrentStackReleaseId, manualStack.ComposeFile)
            .ToDictionary(
                item => item.Key,
                item => new StackDesiredService(
                    item.Value.ServiceName,
                    item.Value.Image,
                    item.Value.ExpectedConfigHash),
                StringComparer.OrdinalIgnoreCase);

        return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
    }

    private string? TryGetCurrentSourceRoot(Guid stackId)
    {
        var stackRoot = Path.Combine(stackStoragePathProvider.StacksRoot, stackId.ToString("D"));
        var currentPath = Path.Combine(stackRoot, "current");
        if (Directory.Exists(currentPath))
        {
            return Path.GetFullPath(currentPath);
        }

        var pointerPath = Path.Combine(stackRoot, "current.source");
        if (!File.Exists(pointerPath))
        {
            return null;
        }

        var snapshotRoot = File.ReadAllText(pointerPath).Trim();
        if (string.IsNullOrWhiteSpace(snapshotRoot) || !Directory.Exists(snapshotRoot))
        {
            return null;
        }

        return Path.GetFullPath(snapshotRoot);
    }

    private static string? ResolveRepositoryFile(string snapshotRoot, string relativePath)
    {
        if (string.IsNullOrWhiteSpace(relativePath) || Path.IsPathRooted(relativePath))
        {
            return null;
        }

        var normalizedRoot = Path.GetFullPath(snapshotRoot);
        var fullPath = Path.GetFullPath(Path.Combine(normalizedRoot, relativePath));
        if (!fullPath.StartsWith(normalizedRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar, StringComparison.Ordinal)
            && !string.Equals(fullPath, normalizedRoot, StringComparison.Ordinal))
        {
            return null;
        }

        return File.Exists(fullPath) ? fullPath : null;
    }
}

internal sealed class DockerStackRuntimeStateProvider(
    IPlatformContainerCache platformCache,
    IConnectorFactory<IContainerConnector> connectorFactory) : IStackRuntimeStateProvider
{
    public async Task<StackRuntimeState> GetRuntimeStateAsync(StackDriftStack stack, CancellationToken cancellationToken)
    {
        if (!platformCache.TryGetCacheEntry(stack.PlatformId, out var platform, out var error))
        {
            throw new InvalidOperationException(error?.Message ?? "Platform not found or disconnected.");
        }

        var projectName = StackProjectNameResolver.Resolve(stack);
        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var listResult = await connector.ListContainersAsync(
            StackContainerOwnership.CreateOwnedContainerFilter(platform.Address, projectName, stack.Id),
            cancellationToken);

        if (listResult.IsFailure(out var listError, out var containers))
        {
            throw new InvalidOperationException(listError.Message);
        }

        var runtimeContainers = new List<StackRuntimeContainer>();
        foreach (var container in containers.Values)
        {
            var inspectResult = await connector.InspectAsync(
                new InspectContainerCommand(platform.Address, container.Id),
                cancellationToken);

            if (inspectResult.IsFailure(out _, out var inspect))
            {
                continue;
            }

            var labels = inspect.Config?.Labels ?? new Dictionary<string, string>();
            if (!StackContainerOwnership.IsOwnedByStack(labels, stack.Id))
            {
                continue;
            }

            labels.TryGetValue(ComposeLabels.Service, out var serviceName);
            labels.TryGetValue(CitadelLabels.ServiceHash, out var configHash);

            runtimeContainers.Add(new StackRuntimeContainer(
                ContainerId: container.Id,
                Name: container.Name,
                ServiceName: string.IsNullOrWhiteSpace(serviceName) ? container.Name : serviceName,
                Image: inspect.Config?.Image ?? container.Image,
                State: inspect.State?.Status ?? container.State,
                HealthStatus: inspect.State?.Health?.Status,
                ConfigHash: configHash,
                Labels: labels));
        }

        return new StackRuntimeState(
            PlatformId: platform.Id,
            PlatformAddress: platform.Address,
            PlatformConnectorType: platform.ConnectorType,
            ProjectName: projectName,
            Containers: runtimeContainers);
    }
}

internal sealed class StackDriftChecker(
    IServiceScopeFactory scopeFactory,
    IStackDesiredStateProvider desiredStateProvider,
    IStackRuntimeStateProvider runtimeStateProvider) : IStackDriftChecker
{
    public async Task<StackDriftReport> CheckAsync(Guid stackId, CancellationToken cancellationToken)
    {
        var stack = await LoadStackAsync(stackId, cancellationToken);
        return await CheckAsync(stack, cancellationToken);
    }

    public async Task<StackDriftReport> CheckAsync(StackDriftStack stack, CancellationToken cancellationToken)
    {
        if (stack.Spec is not ManualStack and not GitStack)
        {
            return NoDrift(stack);
        }

        if (stack.Status is StackReleaseStatus.Stopped or StackReleaseStatus.Paused)
        {
            return NoDrift(stack);
        }

        var desired = await desiredStateProvider.GetDesiredStateAsync(stack, cancellationToken);
        var runtime = await runtimeStateProvider.GetRuntimeStateAsync(stack, cancellationToken);
        var drifts = FindDrifts(desired, runtime);

        return new StackDriftReport(
            StackId: stack.Id,
            PlatformId: stack.PlatformId,
            HasDrift: drifts.Count > 0,
            HasAutoFixableDrift: drifts.Any(StackDriftHelpers.IsAutoFixable),
            HasStructuralDrift: drifts.Any(StackDriftHelpers.IsStructural),
            Drifts: drifts);
    }

    private static StackDriftReport NoDrift(StackDriftStack stack)
        => new(
            StackId: stack.Id,
            PlatformId: stack.PlatformId,
            HasDrift: false,
            HasAutoFixableDrift: false,
            HasStructuralDrift: false,
            Drifts: []);

    private async Task<StackDriftStack> LoadStackAsync(Guid stackId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await unitOfWork.Stacks.GetDriftStackAsync(stackId, cancellationToken)
            ?? throw new InvalidOperationException("The provided stack does not exist.");
    }

    private static IReadOnlyList<StackDrift> FindDrifts(StackDesiredState desired, StackRuntimeState runtime)
    {
        var drifts = new List<StackDrift>();
        var actualByService = runtime.Containers
            .GroupBy(x => x.ServiceName, StringComparer.OrdinalIgnoreCase)
            .ToDictionary(x => x.Key, x => x.ToArray(), StringComparer.OrdinalIgnoreCase);

        foreach (var service in desired.Services.Values)
        {
            if (!actualByService.TryGetValue(service.ServiceName, out var containers) || containers.Length == 0)
            {
                drifts.Add(new MissingContainer(service.ServiceName));
                continue;
            }

            foreach (var container in containers)
            {
                if (IsStopped(container.State))
                {
                    drifts.Add(new ContainerStopped(container.ContainerId, service.ServiceName));
                }
                else if (container.State == ContainerStateStatus.Paused)
                {
                    drifts.Add(new ContainerPaused(container.ContainerId, service.ServiceName));
                }

                if (!string.IsNullOrWhiteSpace(container.HealthStatus)
                    && !container.HealthStatus.Equals("healthy", StringComparison.OrdinalIgnoreCase)
                    && !container.HealthStatus.Equals("none", StringComparison.OrdinalIgnoreCase)
                    && !container.HealthStatus.Equals("starting", StringComparison.OrdinalIgnoreCase))
                {
                    drifts.Add(new ContainerUnhealthy(container.ContainerId, service.ServiceName, container.HealthStatus));
                }

                if (!string.IsNullOrWhiteSpace(service.ExpectedConfigHash)
                    && !string.Equals(service.ExpectedConfigHash, container.ConfigHash, StringComparison.Ordinal))
                {
                    drifts.Add(new ConfigHashMismatch(service.ServiceName, service.ExpectedConfigHash, container.ConfigHash));
                }
            }
        }

        foreach (var container in runtime.Containers)
        {
            if (!desired.Services.ContainsKey(container.ServiceName))
            {
                drifts.Add(new ExtraContainer(container.ContainerId, container.ServiceName));
            }
        }

        return drifts;
    }

    private static bool IsStopped(ContainerStateStatus state)
        => state is ContainerStateStatus.Exited
            or ContainerStateStatus.Dead
            or ContainerStateStatus.Offline;
}

internal sealed class StackReconciler(
    IServiceScopeFactory scopeFactory,
    IStackDriftChecker driftChecker,
    IPlatformContainerCache platformCache,
    INotificationQueue notificationQueue,
    IStackStreamManager stackHub,
    IConnectorFactory<IContainerConnector> connectorFactory) : IStackReconciler
{
    public async Task<StackReconciliationResult> ReconcileAsync(Guid stackId, CancellationToken cancellationToken)
    {
        var stack = await LoadStackAsync(stackId, cancellationToken);
        ValidateCanReconcile(stack);

        var beforeReport = await driftChecker.CheckAsync(stack, cancellationToken);

        if (stack.DriftPolicy.Mode != StackDriftMode.AutoFix)
        {
            throw new InvalidOperationException("Stack sync failed because drift auto-fix is not enabled for this stack.");
        }

        if (!beforeReport.HasDrift)
        {
            return new StackReconciliationResult(stack.Id, StackReconciliationStatus.NoDrift, beforeReport, AfterReport: null, []);
        }

        if (!beforeReport.Drifts.Any(drift => CanApply(stack.DriftPolicy, drift)))
        {
            var status = beforeReport.HasStructuralDrift
                ? StackReconciliationStatus.RequiresReapply
                : StackReconciliationStatus.Partial;

            return new StackReconciliationResult(stack.Id, status, beforeReport, AfterReport: null, []);
        }

        if (!platformCache.TryGetCacheEntry(beforeReport.PlatformId, out var platform, out var error))
        {
            throw new InvalidOperationException(error?.Message ?? "Platform not found or disconnected.");
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var actions = new List<StackReconciliationAction>();
        var processedStack = await MarkProcessingAsync(stack.Id, cancellationToken);
        await NotifyProcessingAsync(processedStack, cancellationToken);

        try
        {
            foreach (var drift in beforeReport.Drifts)
            {
                if (drift is ContainerStopped stopped && stack.DriftPolicy.AutoStartStoppedContainers)
                {
                    actions.Add(await PatchContainer(connector, platform.Address, stopped.ContainerId, stopped.ServiceName, StackReconciliationActionType.StartContainer, ContainerAction.START, cancellationToken));
                }
                else if (drift is ContainerPaused paused && stack.DriftPolicy.AutoResumePausedContainers)
                {
                    actions.Add(await PatchContainer(connector, platform.Address, paused.ContainerId, paused.ServiceName, StackReconciliationActionType.ResumeContainer, ContainerAction.UNPAUSE, cancellationToken));
                }
                else if (drift is ExtraContainer extra && stack.DriftPolicy.RemoveExtraContainers)
                {
                    actions.Add(await RemoveContainer(connector, platform.Address, extra.ContainerId, extra.ServiceName, cancellationToken));
                }
            }

            var afterReport = actions.Count == 0
                ? null
                : await driftChecker.CheckAsync(stack.Id, cancellationToken);
            var status = GetStatus(beforeReport, afterReport, actions);
            await ReleaseProcessingAsync(processedStack, GetReleaseStatus(stack, processedStack.PreviousStatus, afterReport), cancellationToken);

            return new StackReconciliationResult(stack.Id, status, beforeReport, afterReport, actions);
        }
        catch
        {
            await ReleaseProcessingAsync(processedStack, processedStack.PreviousStatus, cancellationToken);
            throw;
        }

        static async Task<StackReconciliationAction> PatchContainer(
            IContainerConnector connector,
            string platformAddress,
            string containerId,
            string serviceName,
            StackReconciliationActionType actionType,
            ContainerAction containerAction,
            CancellationToken cancellationToken)
        {
            var result = await connector.PatchAsync(
                new PatchContainerCommand(containerAction, platformAddress, [containerId]),
                cancellationToken);

            return result.IsFailure(out var patchError)
                ? new StackReconciliationAction(containerId, serviceName, actionType, Succeeded: false, patchError.Message)
                : new StackReconciliationAction(containerId, serviceName, actionType, Succeeded: true);
        }

        static async Task<StackReconciliationAction> RemoveContainer(
            IContainerConnector connector,
            string platformAddress,
            string containerId,
            string serviceName,
            CancellationToken cancellationToken)
        {
            var result = await connector.DeleteAsync(
                new DeleteContainerCommand([containerId], platformAddress, Volume: false, Force: true, Link: false),
                cancellationToken);

            return result.IsFailure(out var deleteError)
                ? new StackReconciliationAction(containerId, serviceName, StackReconciliationActionType.RemoveContainer, Succeeded: false, deleteError.Message)
                : new StackReconciliationAction(containerId, serviceName, StackReconciliationActionType.RemoveContainer, Succeeded: true);
        }
    }

    private async Task<StackDriftStack> LoadStackAsync(Guid stackId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await unitOfWork.Stacks.GetDriftStackAsync(stackId, cancellationToken)
            ?? throw new InvalidOperationException("The provided stack does not exist.");
    }

    private async Task<ProcessedStack> MarkProcessingAsync(Guid stackId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var userContext = scope.ServiceProvider.GetRequiredService<IUserContextAccessor>();
        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;

        var stack = await unitOfWork.Stacks.GetAsync(stackId, cancellationToken)
            ?? throw new InvalidOperationException("The provided stack does not exist.");

        if (stack.CurrentStackRelease is null)
        {
            throw new InvalidOperationException("The provided stack does not have a current release.");
        }

        var previousStatus = stack.CurrentStackRelease.Status;
        if (!stack.MarkProcessing(actorId))
        {
            throw new InvalidOperationException("The stack is already being processed.");
        }

        var affectedRow = await unitOfWork.Stacks.UpdateProcessingAsync(
            stack.Id,
            stack.CurrentStackRelease.Status,
            stack.ControlState,
            stack.ControlStartedAt,
            stack.RowVersion,
            checkRowVersion: true,
            actorId,
            cancellationToken);

        if (!affectedRow)
        {
            throw new InvalidOperationException("The stack is already being processed.");
        }

        await unitOfWork.CommitAsync(cancellationToken);
        return new ProcessedStack(stack, previousStatus, actorId);
    }

    private async Task ReleaseProcessingAsync(ProcessedStack processedStack, StackReleaseStatus status, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        processedStack.Stack.ReleaseProcessing(status);
        await unitOfWork.Stacks.UpdateProcessingAsync(
            processedStack.Stack.Id,
            status,
            processedStack.Stack.ControlState,
            processedStack.Stack.ControlStartedAt,
            processedStack.Stack.RowVersion,
            checkRowVersion: false,
            processedStack.ActorId,
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        await NotifyProcessingAsync(processedStack, cancellationToken);
    }

    private Task NotifyProcessingAsync(ProcessedStack processedStack, CancellationToken cancellationToken)
        => notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackHub, processedStack.Stack), cancellationToken).AsTask();

    private static bool CanApply(StackDriftPolicy policy, StackDrift drift)
        => drift switch
        {
            ContainerStopped => policy.AutoStartStoppedContainers,
            ContainerPaused => policy.AutoResumePausedContainers,
            ExtraContainer => policy.RemoveExtraContainers,
            _ => false
        };

    private static void ValidateCanReconcile(StackDriftStack stack)
    {
        if (stack.ControlState == ResourceControlState.Processing || stack.Status is StackReleaseStatus.Applying or StackReleaseStatus.Pending)
        {
            throw new InvalidOperationException("The stack is currently applying and cannot be reconciled.");
        }
    }

    private static StackReconciliationStatus GetStatus(
        StackDriftReport beforeReport,
        StackDriftReport? afterReport,
        List<StackReconciliationAction> actions)
    {
        if (afterReport?.HasStructuralDrift == true || (afterReport is null && beforeReport.HasStructuralDrift))
            return StackReconciliationStatus.RequiresReapply;

        if (afterReport is { HasDrift: false } && actions.Count > 0 && actions.All(x => x.Succeeded))
            return StackReconciliationStatus.Reconciled;

        if (actions.Any(x => !x.Succeeded))
            return StackReconciliationStatus.Partial;

        if (actions.Count == 0)
            return StackReconciliationStatus.Partial;

        if (afterReport is { HasDrift: true })
            return StackReconciliationStatus.Partial;

        return actions.All(x => x.Succeeded)
            ? StackReconciliationStatus.Reconciled
            : StackReconciliationStatus.Partial;
    }

    private static StackReleaseStatus GetReleaseStatus(
        StackDriftStack stack,
        StackReleaseStatus previousStatus,
        StackDriftReport? afterReport)
    {
        if (afterReport is { HasDrift: false })
            return StackReleaseStatus.Healthy;

        if (afterReport is { HasDrift: true } && stack.DriftPolicy.MarkDegraded)
            return StackReleaseStatus.Degraded;

        return previousStatus;
    }

    private sealed record ProcessedStack(Stack Stack, StackReleaseStatus PreviousStatus, Guid ActorId);
}

internal static class StackDriftHelpers
{
    public static bool IsAutoFixable(StackDrift drift)
        => drift is ContainerStopped or ContainerPaused or ExtraContainer;

    public static bool IsStructural(StackDrift drift)
        => drift is MissingContainer
            or ImageMismatch
            or ConfigHashMismatch;

    public static AlertSeverity GetSeverity(StackDriftReport report)
        => report.Drifts.Any(x => x is MissingContainer or ContainerUnhealthy or ImageMismatch or ConfigHashMismatch)
            ? AlertSeverity.Critical
            : AlertSeverity.Warning;

    public static string Fingerprint(StackDriftReport report)
    {
        var lines = report.Drifts
            .Select(Normalize)
            .Order(StringComparer.Ordinal)
            .ToArray();

        var raw = $"stack:{report.StackId}:drift:{string.Join('|', lines)}";
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(raw));
        return $"stack:{report.StackId}:drift:{Convert.ToHexString(hash)}";
    }

    public static IReadOnlyList<string> Summaries(StackDriftReport report)
        => report.Drifts.Select(Summarize).ToArray();

    public static string ShortSummary(StackDriftReport report)
        => string.Join("; ", Summaries(report).Take(3));

    public static string Summarize(StackDrift drift)
        => drift switch
        {
            MissingContainer x => $"service '{x.ServiceName}' is missing",
            ExtraContainer x => $"extra container '{x.ContainerId}' for service '{x.ServiceName}'",
            ContainerStopped x => $"container '{x.ContainerId}' for service '{x.ServiceName}' is stopped",
            ContainerPaused x => $"container '{x.ContainerId}' for service '{x.ServiceName}' is paused",
            ContainerUnhealthy x => $"container '{x.ContainerId}' for service '{x.ServiceName}' is unhealthy ({x.HealthStatus})",
            ImageMismatch x => $"service '{x.ServiceName}' image mismatch: expected '{x.ExpectedImage}', actual '{x.ActualImage}'",
            ConfigHashMismatch x => $"service '{x.ServiceName}' config hash mismatch",
            _ => "unknown drift"
        };

    private static string Normalize(StackDrift drift)
        => drift switch
        {
            MissingContainer x => $"missing:{x.ServiceName}",
            ExtraContainer x => $"extra:{x.ServiceName}:{x.ContainerId}",
            ContainerStopped x => $"stopped:{x.ServiceName}:{x.ContainerId}",
            ContainerPaused x => $"paused:{x.ServiceName}:{x.ContainerId}",
            ContainerUnhealthy x => $"unhealthy:{x.ServiceName}:{x.ContainerId}:{x.HealthStatus}",
            ImageMismatch x => $"image:{x.ServiceName}:{x.ExpectedImage}:{x.ActualImage}",
            ConfigHashMismatch x => $"config:{x.ServiceName}:{x.ExpectedHash}:{x.ActualHash}",
            _ => drift.ToString() ?? string.Empty
        };
}

internal sealed record StackDesiredState(
    string ProjectName,
    IReadOnlyDictionary<string, StackDesiredService> Services);

internal sealed record StackDesiredService(
    string ServiceName,
    string? Image,
    string? ExpectedConfigHash);

internal sealed record StackRuntimeState(
    Guid PlatformId,
    string PlatformAddress,
    PlatformConnectorType PlatformConnectorType,
    string ProjectName,
    IReadOnlyList<StackRuntimeContainer> Containers);

internal sealed record StackRuntimeContainer(
    string ContainerId,
    string Name,
    string ServiceName,
    string? Image,
    ContainerStateStatus State,
    string? HealthStatus,
    string? ConfigHash,
    IReadOnlyDictionary<string, string> Labels);
