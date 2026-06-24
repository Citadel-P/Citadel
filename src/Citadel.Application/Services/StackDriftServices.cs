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
using YamlDotNet.RepresentationModel;

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

internal sealed class ManualStackDesiredStateProvider : IStackDesiredStateProvider
{
    public Task<StackDesiredState> GetDesiredStateAsync(StackDriftStack stack, CancellationToken cancellationToken)
    {
        if (stack.Spec is not ManualStack manualStack)
        {
            return Task.FromResult(new StackDesiredState(
                ProjectName: StackProjectNameResolver.Resolve(stack),
                Services: new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase)));
        }

        return Task.FromResult(Parse(stack, manualStack));
    }

    private static StackDesiredState Parse(StackDriftStack stack, ManualStack manualStack)
    {
        var services = new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase);

        if (string.IsNullOrWhiteSpace(manualStack.ComposeFile))
        {
            return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
        }

        var managedComposeFile = StackComposeLabelInjector.Inject(
            manualStack.ComposeFile,
            stack.Id,
            stack.CurrentStackReleaseId);

        using var reader = new StringReader(managedComposeFile);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode root)
        {
            return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
        }

        if (!TryGetMapping(root, "services", out var servicesNode))
        {
            return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
        }

        foreach (var (key, value) in servicesNode.Children)
        {
            if (key is not YamlScalarNode serviceKey || string.IsNullOrWhiteSpace(serviceKey.Value))
                continue;

            var image = value is YamlMappingNode serviceNode
                ? TryGetScalar(serviceNode, "image")
                : null;

            var expectedConfigHash = value is YamlMappingNode node
                ? TryGetServiceLabel(node, CitadelLabels.ServiceHash)
                : null;

            services[serviceKey.Value] = new StackDesiredService(
                ServiceName: serviceKey.Value,
                Image: image,
                ExpectedConfigHash: expectedConfigHash);
        }

        return new StackDesiredState(StackProjectNameResolver.Resolve(stack), services);
    }

    private static bool TryGetMapping(YamlMappingNode node, string key, out YamlMappingNode value)
    {
        foreach (var (childKey, childValue) in node.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase)
                && childValue is YamlMappingNode mapping)
            {
                value = mapping;
                return true;
            }
        }

        value = null!;
        return false;
    }

    private static string? TryGetScalar(YamlMappingNode node, string key)
    {
        foreach (var (childKey, childValue) in node.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase)
                && childValue is YamlScalarNode value)
            {
                return value.Value;
            }
        }

        return null;
    }

    private static string? TryGetServiceLabel(YamlMappingNode serviceNode, string labelName)
    {
        foreach (var (key, value) in serviceNode.Children)
        {
            if (key is not YamlScalarNode scalar || !string.Equals(scalar.Value, "labels", StringComparison.OrdinalIgnoreCase))
                continue;

            return value switch
            {
                YamlMappingNode mapping => TryGetScalar(mapping, labelName),
                YamlSequenceNode sequence => TryGetSequenceLabel(sequence, labelName),
                _ => null
            };
        }

        return null;
    }

    private static string? TryGetSequenceLabel(YamlSequenceNode sequence, string labelName)
    {
        foreach (var child in sequence.Children.OfType<YamlScalarNode>())
        {
            var value = child.Value;
            if (value is null || !value.StartsWith(labelName, StringComparison.OrdinalIgnoreCase))
                continue;

            if (value.Length == labelName.Length)
                return string.Empty;

            if (value[labelName.Length] == '=')
                return value[(labelName.Length + 1)..];
        }

        return null;
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

internal sealed class ManualStackDriftChecker(
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
        if (stack.StackSource != StackSource.WebEditor)
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
                    && !container.HealthStatus.Equals("none", StringComparison.OrdinalIgnoreCase))
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
