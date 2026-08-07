using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal interface ISwarmReconciliationCoordinator
{
    Task<Result> RefreshAsync(Guid platformId, CancellationToken cancellationToken);
    Task<Result> EnsureInitializedAsync(Guid platformId, CancellationToken cancellationToken);
    ValueTask NotifyDaemonEventAsync(
        Guid platformId,
        DaemonEventInfo daemonEvent,
        CancellationToken cancellationToken);
}

internal sealed class SwarmReconciliationJob(
    IServiceScopeFactory scopeFactory,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IDbWorkQueue dbQueue,
    INotificationQueue notificationQueue,
    IApplicationHubDispatcher hubDispatcher,
    TimeProvider timeProvider,
    ILogger<SwarmReconciliationJob> logger) : BackgroundService, ISwarmReconciliationCoordinator
{
    private const int MaximumConcurrentRefreshes = 4;
    private const int PlatformGateCount = 64;
    private const int EventQueueCapacity = 256;
    private static readonly TimeSpan Interval = TimeSpan.FromMinutes(30);
    private static readonly TimeSpan EventDebounce = TimeSpan.FromSeconds(2);
    private static readonly TimeSpan RefreshTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan NotificationEnqueueTimeout = TimeSpan.FromSeconds(5);
    private readonly SemaphoreSlim _refreshConcurrency = new(MaximumConcurrentRefreshes, MaximumConcurrentRefreshes);
    private readonly SemaphoreSlim[] _platformGates = CreatePlatformGates();
    private readonly ConcurrentDictionary<Guid, byte> _knownSwarmPlatforms = new();
    private readonly ConcurrentDictionary<Guid, byte> _pendingRefreshes = new();
    private readonly Channel<Guid> _refreshRequests = Channel.CreateBounded<Guid>(
        new BoundedChannelOptions(EventQueueCapacity)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });

    public Task<Result> RefreshAsync(Guid platformId, CancellationToken cancellationToken) =>
        RefreshAsync(platformId, onlyWhenEmpty: false, ignoreMissingOrNonSwarm: false, cancellationToken);

    public Task<Result> EnsureInitializedAsync(Guid platformId, CancellationToken cancellationToken) =>
        RefreshAsync(platformId, onlyWhenEmpty: true, ignoreMissingOrNonSwarm: false, cancellationToken);

    public ValueTask NotifyDaemonEventAsync(
        Guid platformId,
        DaemonEventInfo daemonEvent,
        CancellationToken cancellationToken)
    {
        var shouldRefresh = daemonEvent switch
        {
            DaemonResourceEventInfo
            {
                Type: ContainerEventType.Service or
                    ContainerEventType.Node or
                    ContainerEventType.Secret or
                    ContainerEventType.Config
            } => true,
            DaemonResourceEventInfo
            {
                Type: ContainerEventType.Network,
                Scope: DaemonEventScope.Swarm
            } => true,
            DaemonNetworkEventInfo { Scope: DaemonEventScope.Swarm } => true,
            DaemonContainerEventInfo => _knownSwarmPlatforms.ContainsKey(platformId),
            _ => false
        };

        return shouldRefresh
            ? QueueRefreshAsync(platformId, cancellationToken)
            : ValueTask.CompletedTask;
    }

    private async Task<Result> RefreshAsync(
        Guid platformId,
        bool onlyWhenEmpty,
        bool ignoreMissingOrNonSwarm,
        CancellationToken cancellationToken)
    {
        var platformGate = GetPlatformGate(platformId);
        await platformGate.WaitAsync(cancellationToken);
        try
        {
            await _refreshConcurrency.WaitAsync(cancellationToken);
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
                if (platform is null)
                    return ignoreMissingOrNonSwarm
                        ? Result.Success()
                        : Result.Failure(new NotFoundError("Platform does not exist."));
                if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
                {
                    _knownSwarmPlatforms.TryRemove(platformId, out _);
                    return ignoreMissingOrNonSwarm
                        ? Result.Success()
                        : Result.Failure(new BadRequestError("Swarm inventory is only available for Docker Swarm platforms."));
                }

                _knownSwarmPlatforms.TryAdd(platformId, 0);
                if (onlyWhenEmpty
                    && (await unitOfWork.Swarm.GetNodesAsync(platformId, cancellationToken)).Count != 0)
                    return Result.Success();

                using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
                timeout.CancelAfter(RefreshTimeout);
                var connector = connectorFactory.GetConnector(platform.ConnectorType);
                var nodesTask = connector.ListNodesAsync(
                    new ListSwarmNodesCommand(platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems, IncludeTaskCounts: true),
                    timeout.Token);
                var servicesTask = connector.ListServicesAsync(new ListSwarmServicesCommand(
                    platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems), timeout.Token);
                var tasksTask = connector.ListTasksAsync(new ListSwarmTasksCommand(platform.Address), timeout.Token);
                var networksTask = connector.ListNetworksAsync(new ListSwarmNetworksCommand(
                    platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems), timeout.Token);
                var secretsTask = connector.ListSecretsAsync(new ListSwarmSecretsCommand(
                    platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems), timeout.Token);
                var configsTask = connector.ListConfigsAsync(new ListSwarmConfigsCommand(
                    platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems), timeout.Token);
                await Task.WhenAll(nodesTask, servicesTask, tasksTask, networksTask, secretsTask, configsTask);

                var nodesResult = await nodesTask;
                var servicesResult = await servicesTask;
                var tasksResult = await tasksTask;
                var networksResult = await networksTask;
                var secretsResult = await secretsTask;
                var configsResult = await configsTask;
                if (!nodesResult.IsSuccess(out var nodes, out var error)
                    || !servicesResult.IsSuccess(out var services, out error)
                    || !tasksResult.IsSuccess(out var tasks, out error)
                    || !networksResult.IsSuccess(out var networks, out error)
                    || !secretsResult.IsSuccess(out var secrets, out error)
                    || !configsResult.IsSuccess(out var configs, out error))
                {
                    await TryPersistStaleAsync(platformId, cancellationToken);
                    return Result.Failure(error!);
                }

                var snapshot = BuildSnapshot(platformId, nodes, services, tasks, networks, secrets, configs);
                await PersistAsync(platformId, snapshot, timeout.Token);
                return Result.Success();
            }
            finally
            {
                _refreshConcurrency.Release();
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException)
        {
            await TryPersistStaleAsync(platformId, cancellationToken);
            return Result.Failure(new InternalServerError("Timed out while reading Swarm inventory."));
        }
        catch (Exception exception)
        {
            logger.LogError(exception, "Failed to reconcile Swarm inventory for platform {PlatformId}.", platformId);
            await TryPersistStaleAsync(platformId, cancellationToken);
            return Result.Failure(new InternalServerError("Failed to reconcile Swarm inventory."));
        }
        finally
        {
            platformGate.Release();
        }
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var eventDrivenTask = RunEventDrivenAsync(stoppingToken);
        var periodicTask = RunPeriodicAsync(stoppingToken);
        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    private async Task RunPeriodicAsync(CancellationToken stoppingToken)
    {
        await ReconcileOnceAsync(stoppingToken);
        using var timer = new PeriodicTimer(Interval);
        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
                await ReconcileOnceAsync(stoppingToken);
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
        {
        }
    }

    private async Task RunEventDrivenAsync(CancellationToken cancellationToken)
    {
        try
        {
            while (await _refreshRequests.Reader.WaitToReadAsync(cancellationToken))
            {
                await Task.Delay(EventDebounce, timeProvider, cancellationToken);
                await ReconcilePendingAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
    }

    internal async Task ReconcilePendingAsync(CancellationToken cancellationToken)
    {
        var platformIds = new HashSet<Guid>();
        while (_refreshRequests.Reader.TryRead(out var platformId))
        {
            _pendingRefreshes.TryRemove(platformId, out _);
            platformIds.Add(platformId);
        }

        await Parallel.ForEachAsync(
            platformIds,
            new ParallelOptions
            {
                MaxDegreeOfParallelism = MaximumConcurrentRefreshes,
                CancellationToken = cancellationToken
            },
            async (platformId, ct) =>
            {
                var result = await RefreshAsync(
                    platformId,
                    onlyWhenEmpty: false,
                    ignoreMissingOrNonSwarm: true,
                    ct);
                if (result.IsFailure(out var error))
                    logger.LogWarning(
                        "Event-triggered Swarm reconciliation failed for platform {PlatformId}: {Error}",
                        platformId,
                        error?.Message);
            });
    }

    internal async Task ReconcileOnceAsync(CancellationToken cancellationToken)
    {
        try
        {
            await ReconcileAllAsync(cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogError(exception, "Failed to enumerate Docker Swarm platforms for reconciliation.");
        }
    }

    private async Task ReconcileAllAsync(CancellationToken cancellationToken)
    {
        IReadOnlyList<Guid> platformIds;
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken) ?? [];
            platformIds = platforms
                .Where(static platform => platform.PlatformDescriptor is DockerSwarmPlatformDescriptor)
                .Select(static platform => platform.Id)
                .ToArray();
        }

        var currentPlatformIds = platformIds.ToHashSet();
        foreach (var knownPlatformId in _knownSwarmPlatforms.Keys)
        {
            if (!currentPlatformIds.Contains(knownPlatformId))
                _knownSwarmPlatforms.TryRemove(knownPlatformId, out _);
        }
        foreach (var platformId in platformIds)
            _knownSwarmPlatforms.TryAdd(platformId, 0);

        await Parallel.ForEachAsync(
            platformIds,
            new ParallelOptions
            {
                MaxDegreeOfParallelism = MaximumConcurrentRefreshes,
                CancellationToken = cancellationToken
            },
            async (platformId, ct) =>
            {
                var result = await RefreshAsync(platformId, ct);
                if (result.IsFailure(out var error))
                    logger.LogWarning("Swarm reconciliation failed for platform {PlatformId}: {Error}", platformId, error?.Message);
            });
    }

    private SwarmProjectionSnapshot BuildSnapshot(
        Guid platformId,
        IReadOnlyList<SwarmNodeResult> nodes,
        IReadOnlyList<SwarmServiceResult> services,
        IReadOnlyList<SwarmTaskResult> tasks,
        IReadOnlyList<SwarmNetworkResult> networks,
        IReadOnlyList<SwarmSecretResult> secrets,
        IReadOnlyList<SwarmConfigResult> configs)
    {
        var observedAt = timeProvider.GetUtcNow();
        var nodeProjections = new SwarmNodeProjection[nodes.Count];
        var nodeNames = new Dictionary<string, string>(nodes.Count, StringComparer.Ordinal);
        for (var index = 0; index < nodes.Count; index++)
        {
            nodeProjections[index] = SwarmNodeProjection.FromObservation(platformId, nodes[index], observedAt);
            nodeNames[nodes[index].Id] = nodes[index].Hostname;
        }

        var serviceProjections = new SwarmServiceProjection[services.Count];
        var serviceNames = new Dictionary<string, string>(services.Count, StringComparer.Ordinal);
        for (var index = 0; index < services.Count; index++)
        {
            serviceProjections[index] = SwarmServiceProjection.FromObservation(platformId, services[index], observedAt);
            serviceNames[services[index].Id] = services[index].Name;
        }
        var networkServices = BuildReferences(services, static service => service.NetworkIds);
        var secretServices = BuildReferences(services, static service => service.SecretIds);
        var configServices = BuildReferences(services, static service => service.ConfigIds);

        var taskCount = Math.Min(tasks.Count, SwarmInventoryLimits.MaximumItems);
        var taskProjections = new SwarmTaskProjection[taskCount];
        for (var index = 0; index < taskCount; index++)
        {
            var task = tasks[index];
            serviceNames.TryGetValue(task.ServiceId, out var serviceName);
            nodeNames.TryGetValue(task.NodeId, out var nodeName);
            taskProjections[index] = SwarmTaskProjection.FromObservation(
                platformId, task, serviceName ?? string.Empty, nodeName ?? string.Empty, observedAt);
        }

        var networkProjections = new SwarmNetworkProjection[networks.Count];
        for (var index = 0; index < networks.Count; index++)
            networkProjections[index] = SwarmNetworkProjection.FromObservation(
                platformId, networks[index], GetReferences(networkServices, networks[index].Id, networks[index].Name), observedAt);
        var secretProjections = new SwarmSecretProjection[secrets.Count];
        for (var index = 0; index < secrets.Count; index++)
            secretProjections[index] = SwarmSecretProjection.FromObservation(
                platformId, secrets[index], GetReferences(secretServices, secrets[index].Id), observedAt);
        var configProjections = new SwarmConfigProjection[configs.Count];
        for (var index = 0; index < configs.Count; index++)
            configProjections[index] = SwarmConfigProjection.FromObservation(
                platformId, configs[index], GetReferences(configServices, configs[index].Id), observedAt);

        return new SwarmProjectionSnapshot(
            nodeProjections, serviceProjections, taskProjections, networkProjections,
            secretProjections, configProjections);
    }

    private static Dictionary<string, List<string>> BuildReferences(
        IReadOnlyList<SwarmServiceResult> services,
        Func<SwarmServiceResult, IReadOnlyList<string>> resourceIds)
    {
        var result = new Dictionary<string, List<string>>(StringComparer.Ordinal);
        foreach (var service in services)
        {
            foreach (var resourceId in resourceIds(service))
            {
                if (!result.TryGetValue(resourceId, out var names))
                {
                    names = [];
                    result.Add(resourceId, names);
                }
                names.Add(service.Name);
            }
        }
        return result;
    }

    private static IReadOnlyList<string> GetReferences(Dictionary<string, List<string>> references, string resourceId)
    {
        if (!references.TryGetValue(resourceId, out var names))
            return [];
        names.Sort(StringComparer.OrdinalIgnoreCase);
        return names.ToArray();
    }

    private static IReadOnlyList<string> GetReferences(
        Dictionary<string, List<string>> references,
        string resourceId,
        string resourceName)
    {
        references.TryGetValue(resourceId, out var idReferences);
        references.TryGetValue(resourceName, out var nameReferences);
        if (idReferences is null)
            return nameReferences is null ? [] : SortedCopy(nameReferences);
        if (nameReferences is null || ReferenceEquals(idReferences, nameReferences))
            return SortedCopy(idReferences);

        return idReferences
            .Concat(nameReferences)
            .Distinct(StringComparer.Ordinal)
            .Order(StringComparer.OrdinalIgnoreCase)
            .ToArray();
    }

    private static IReadOnlyList<string> SortedCopy(List<string> names)
    {
        var result = names.ToArray();
        Array.Sort(result, StringComparer.OrdinalIgnoreCase);
        return result;
    }

    private async Task PersistAsync(
        Guid platformId,
        SwarmProjectionSnapshot? snapshot,
        CancellationToken cancellationToken)
    {
        var workItem = new PersistSwarmSnapshotWorkItem(
            platformId,
            snapshot,
            timeProvider.GetUtcNow());
        await dbQueue.EnqueueAndWaitAsync(workItem, cancellationToken);
        await TryNotifyAsync(platformId, workItem.Current, workItem.ManagedChanges, cancellationToken);
    }

    private async Task TryNotifyAsync(
        Guid platformId,
        SwarmProjectionSnapshot snapshot,
        IReadOnlyList<ManagedSwarmServiceChange> managedChanges,
        CancellationToken cancellationToken)
    {
        try
        {
            using var enqueueTimeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            enqueueTimeout.CancelAfter(NotificationEnqueueTimeout);
            await notificationQueue.EnqueueAsync(
                new SwarmInventoryUpdatedNotificationWorkItem(
                    hubDispatcher, platformId, snapshot, managedChanges),
                enqueueTimeout.Token);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogWarning(exception, "Failed to enqueue the Swarm inventory notification for platform {PlatformId}.", platformId);
        }
    }

    private SemaphoreSlim GetPlatformGate(Guid platformId)
    {
        var hash = (uint)platformId.GetHashCode();
        return _platformGates[hash % (uint)PlatformGateCount];
    }

    private ValueTask QueueRefreshAsync(Guid platformId, CancellationToken cancellationToken)
    {
        if (!_pendingRefreshes.TryAdd(platformId, 0))
            return ValueTask.CompletedTask;

        return WriteRefreshRequestAsync(platformId, cancellationToken);
    }

    private async ValueTask WriteRefreshRequestAsync(Guid platformId, CancellationToken cancellationToken)
    {
        try
        {
            await _refreshRequests.Writer.WriteAsync(platformId, cancellationToken);
        }
        catch
        {
            _pendingRefreshes.TryRemove(platformId, out _);
            throw;
        }
    }

    private static SemaphoreSlim[] CreatePlatformGates()
    {
        var gates = new SemaphoreSlim[PlatformGateCount];
        for (var index = 0; index < gates.Length; index++)
            gates[index] = new SemaphoreSlim(1, 1);
        return gates;
    }

    private async Task TryPersistStaleAsync(Guid platformId, CancellationToken cancellationToken)
    {
        try
        {
            await PersistAsync(platformId, snapshot: null, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogError(exception, "Failed to mark Swarm inventory stale for platform {PlatformId}.", platformId);
        }
    }
}

internal sealed class PersistSwarmSnapshotWorkItem(
    Guid platformId,
    SwarmProjectionSnapshot? snapshot,
    DateTimeOffset? reconciliationCompletedAt = null) : IDbWorkItem
{
    private static readonly TimeSpan OutcomeUnknownGrace = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan PendingAcceptanceGrace = TimeSpan.FromMinutes(2.5);
    private static readonly TimeSpan AcceptedObservationGrace = TimeSpan.FromMinutes(2.5);
    public SwarmProjectionSnapshot Current { get; private set; } = null!;
    public IReadOnlyList<ManagedSwarmServiceChange> ManagedChanges { get; private set; } = [];

    public async Task ExecuteAsync(IUnitOfWork unitOfWork, CancellationToken cancellationToken)
    {
        if (snapshot is null)
        {
            await unitOfWork.Swarm.MarkStaleAsync(platformId, cancellationToken);
            Current = new SwarmProjectionSnapshot(
                await unitOfWork.Swarm.GetNodesAsync(platformId, cancellationToken),
                await unitOfWork.Swarm.GetServicesAsync(platformId, cancellationToken),
                await unitOfWork.Swarm.GetTasksAsync(platformId, SwarmInventoryLimits.MaximumItems, cancellationToken),
                await unitOfWork.Swarm.GetNetworksAsync(platformId, cancellationToken),
                await unitOfWork.Swarm.GetSecretsAsync(platformId, cancellationToken),
                await unitOfWork.Swarm.GetConfigsAsync(platformId, cancellationToken));
        }
        else
        {
            Current = await NormalizeManagedServicesAsync(unitOfWork, snapshot, cancellationToken);
            await unitOfWork.Swarm.ReplaceAsync(platformId, Current, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);
    }

    private async Task<SwarmProjectionSnapshot> NormalizeManagedServicesAsync(
        IUnitOfWork unitOfWork,
        SwarmProjectionSnapshot source,
        CancellationToken cancellationToken)
    {
        var changes = new List<ManagedSwarmServiceChange>();
        var managed = await unitOfWork.SwarmServices.GetByPlatformAsync(platformId, cancellationToken);
        var clusterId = managed.Count == 0
            ? null
            : (await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken))?.ClusterId;
        var completedAt = reconciliationCompletedAt ?? DateTimeOffset.UtcNow;
        var byId = managed.ToDictionary(static service => service.Id);
        var claimed = source.Services
            .Where(static service => service.SwarmServiceId is not null)
            .GroupBy(static service => service.SwarmServiceId!.Value)
            .ToDictionary(static group => group.Key, static group => group.ToArray());
        var normalized = new SwarmServiceProjection[source.Services.Count];

        for (var index = 0; index < source.Services.Count; index++)
        {
            var projection = source.Services[index];
            if (projection.SwarmServiceId is not Guid serviceId)
            {
                normalized[index] = projection;
                continue;
            }

            if (!byId.TryGetValue(serviceId, out var owner))
            {
                normalized[index] = projection with
                {
                    Ownership = SwarmServiceOwnership.OwnershipConflict,
                    OwnershipDiagnostic = "The Citadel Service ownership label does not resolve on this platform."
                };
                continue;
            }

            var duplicate = claimed[serviceId].Length != 1;
            var idMismatch = owner.DockerServiceId is not null
                && !string.Equals(owner.DockerServiceId, projection.DockerServiceId, StringComparison.Ordinal);
            normalized[index] = duplicate || idMismatch
                ? projection with
                {
                    Ownership = SwarmServiceOwnership.OwnershipConflict,
                    OwnershipDiagnostic = duplicate
                        ? "Multiple Docker Services claim this Citadel Service."
                        : "The Docker Service identity does not match the Citadel Service."
                }
                : projection with
                {
                    Ownership = SwarmServiceOwnership.CitadelService,
                    OwnershipDiagnostic = null
                };
        }

        var normalizedClaims = normalized
            .Where(static item => item.SwarmServiceId is not null)
            .GroupBy(static item => item.SwarmServiceId!.Value)
            .ToDictionary(static group => group.Key, static group => group.ToArray());

        foreach (var service in managed)
        {
            var previousHealth = service.Health;
            var previousSynchronization = service.SynchronizationState;
            var previousDockerServiceId = service.DockerServiceId;
            var previousDockerVersion = service.DockerVersionIndex;
            var previousControlState = service.ControlState;
            var previousOperation = service.CurrentOperation;
            ActivityEvent? operationActivity = null;
            var matches = normalizedClaims.TryGetValue(service.Id, out var observed) ? observed : [];
            var live = matches.Length == 1 ? matches[0] : null;
            if (matches.Length > 1)
                live = matches[0] with { Ownership = SwarmServiceOwnership.OwnershipConflict };

            var operation = service.CurrentOperation;
            if (operation?.State == SwarmServiceOperationState.Prepared && operation.AttemptedAt is null)
            {
                service.CompleteOperation(
                    SwarmServiceOperationState.Canceled,
                    resultCode: "UndispatchedOperation",
                    resultMessage: "The undispatched operation was canceled during reconciliation.");
                operationActivity = CreateFailureActivity(
                    service,
                    operation,
                    "The undispatched operation was canceled during reconciliation.");
            }
            else if (operation is not null
                && live?.Ownership == SwarmServiceOwnership.OwnershipConflict
                && !IsTerminal(operation.State))
            {
                service.ApplyObservation(live);
                service.CompleteOperation(
                    SwarmServiceOperationState.OwnershipConflict,
                    resultCode: "OwnershipConflict",
                    resultMessage: live.OwnershipDiagnostic ?? "Docker Service ownership is ambiguous.");
                operationActivity = CreateFailureActivity(
                    service,
                    operation,
                    live.OwnershipDiagnostic ?? "Docker Service ownership is ambiguous.");
            }
            else if (operation is not null
                && operation.State is (SwarmServiceOperationState.PendingAcceptance
                    or SwarmServiceOperationState.Accepted
                    or SwarmServiceOperationState.OutcomeUnknown)
                && live is not null
                && live.Labels.TryGetValue("com.citadel.operation-id", out var operationLabel)
                && Guid.TryParse(operationLabel, out var observedOperationId)
                && observedOperationId == operation.Id
                && operation.Kind != SwarmServiceOperationKind.Delete
                && (operation.BaseDockerVersion is null || live.VersionIndex > operation.BaseDockerVersion)
                && (operation.Kind != SwarmServiceOperationKind.ForceUpdate
                    || operation.ExpectedForceUpdate is null
                    || live.ForceUpdate >= operation.ExpectedForceUpdate))
            {
                service.MarkOperationAccepted(live.DockerServiceId, live.VersionIndex);
                service.ApplyObservation(live);
                var targetObserved = string.Equals(
                    live.LiveRuntimeHash,
                    operation.TargetRuntimeHash,
                    StringComparison.Ordinal);
                if (TryGetRolloutFailure(live, source.Tasks, out var resultCode, out var reason))
                {
                    service.CompleteOperation(
                        SwarmServiceOperationState.Rejected,
                        resultCode: resultCode,
                        resultMessage: reason);
                    operationActivity = CreateFailureActivity(service, operation, reason);
                }
                else if (targetObserved && IsRolloutComplete(live))
                {
                    service.CompleteOperation(
                        SwarmServiceOperationState.Completed,
                        live.LiveRuntimeHash,
                        ExtractDigest(live.Image));
                    operationActivity = CreateSuccessActivity(service, operation);
                }
                else if (!targetObserved
                    && TryGetUnresolvedOperationFailure(
                        operation,
                        live,
                        clusterId,
                        completedAt,
                        out resultCode,
                        out reason))
                {
                    service.CompleteOperation(
                        SwarmServiceOperationState.Rejected,
                        resultCode: resultCode,
                        resultMessage: reason);
                    operationActivity = CreateFailureActivity(service, operation, reason);
                }
            }
            else
            {
                service.ApplyObservation(live);
                if (operation is not null
                    && CanProveNotAccepted(operation, live, clusterId, completedAt))
                {
                    const string notAcceptedReason = "A complete Swarm observation proved that Docker did not accept the operation.";
                    service.CompleteOperation(
                        SwarmServiceOperationState.NotAccepted,
                        resultCode: "NotAccepted",
                        resultMessage: notAcceptedReason);
                    operationActivity = CreateFailureActivity(service, operation, notAcceptedReason);
                }
                else if (operation is not null
                    && TryGetUnresolvedOperationFailure(operation, live, clusterId, completedAt, out var resultCode, out var reason))
                {
                    service.CompleteOperation(
                        SwarmServiceOperationState.Rejected,
                        resultCode: resultCode,
                        resultMessage: reason);
                    operationActivity = CreateFailureActivity(service, operation, reason);
                }
            }

            if (operation is not null && IsDeleteSatisfied(operation, live))
            {
                service.CompleteOperation(SwarmServiceOperationState.Completed);
                if (await unitOfWork.SwarmServices.RemoveAsync(
                        service.Id, service.RowVersion, cancellationToken) > 0)
                {
                    await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
                        service.PlatformId,
                        service.Id,
                        operation.ActorId ?? Constants.SystemId,
                        service.Name,
                        ActivityEventType.SwarmServiceDeleted,
                        ActivityStatus.Information,
                        new SwarmServiceDeleted(service.ToActivitySnapshot())), cancellationToken);
                    changes.Add(new ManagedSwarmServiceChange(service, "delete"));
                }
                continue;
            }

            if (previousHealth != service.Health
                || previousSynchronization != service.SynchronizationState
                || previousDockerServiceId != service.DockerServiceId
                || previousDockerVersion != service.DockerVersionIndex
                || previousControlState != service.ControlState
                || previousOperation != service.CurrentOperation)
            {
                if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) > 0)
                {
                    if (operationActivity is not null)
                        await unitOfWork.ActivityEventRepository.AddAsync(operationActivity, cancellationToken);
                    changes.Add(new ManagedSwarmServiceChange(service, "update"));
                }
            }
        }

        ManagedChanges = changes;
        return source with { Services = normalized };
    }

    internal static bool CanProveNotAccepted(
        SwarmServiceOperation operation,
        SwarmServiceProjection? live,
        string? clusterId,
        DateTimeOffset completedAt)
    {
        if (string.IsNullOrWhiteSpace(operation.ClusterId)
            || !string.Equals(operation.ClusterId, clusterId, StringComparison.Ordinal)
            || operation.AttemptedAt is not DateTime attemptedAt)
            return false;

        var barrier = operation.State switch
        {
            SwarmServiceOperationState.OutcomeUnknown when operation.CompletedAt is DateTime outcomeAt =>
                new DateTimeOffset(outcomeAt, TimeSpan.Zero) + OutcomeUnknownGrace,
            SwarmServiceOperationState.PendingAcceptance =>
                new DateTimeOffset(attemptedAt, TimeSpan.Zero) + PendingAcceptanceGrace,
            _ => DateTimeOffset.MaxValue
        };
        if (completedAt < barrier)
            return false;

        if (operation.Kind == SwarmServiceOperationKind.Delete)
            return live is not null
                && operation.BaseDockerVersion is long deleteVersion
                && live.VersionIndex == deleteVersion;

        if (operation.BaseDockerVersion is null)
            return live is null;

        return live is not null
            && live.VersionIndex == operation.BaseDockerVersion
            && (!live.Labels.TryGetValue("com.citadel.operation-id", out var operationLabel)
                || !Guid.TryParse(operationLabel, out var observedOperationId)
                || observedOperationId != operation.Id);
    }

    internal static bool TryGetUnresolvedOperationFailure(
        SwarmServiceOperation operation,
        SwarmServiceProjection? live,
        string? clusterId,
        DateTimeOffset completedAt,
        out string resultCode,
        out string reason)
    {
        resultCode = string.Empty;
        reason = string.Empty;
        if (operation.AttemptedAt is not DateTime attemptedAt
            || string.IsNullOrWhiteSpace(operation.ClusterId)
            || !string.Equals(operation.ClusterId, clusterId, StringComparison.Ordinal))
        {
            return false;
        }

        var barrier = operation.State switch
        {
            SwarmServiceOperationState.PendingAcceptance or SwarmServiceOperationState.Accepted =>
                new DateTimeOffset(attemptedAt, TimeSpan.Zero) + AcceptedObservationGrace,
            SwarmServiceOperationState.OutcomeUnknown when operation.CompletedAt is DateTime outcomeAt =>
                new DateTimeOffset(outcomeAt, TimeSpan.Zero) + AcceptedObservationGrace,
            _ => DateTimeOffset.MaxValue
        };
        if (completedAt < barrier)
            return false;

        if (operation.Kind == SwarmServiceOperationKind.Delete)
        {
            if (live is null)
                return false;

            resultCode = "DeleteStillPresentAfterAcceptance";
            reason = "Docker accepted the delete, but the Service is still present after the observation grace period.";
            return true;
        }

        if (live is null)
        {
            resultCode = "RuntimeMissingAfterAcceptance";
            reason = "Docker accepted the operation, but the Service was not present after the observation grace period.";
            return true;
        }

        if (!live.Labels.TryGetValue("com.citadel.operation-id", out var operationLabel)
            || !Guid.TryParse(operationLabel, out var observedOperationId)
            || observedOperationId != operation.Id)
        {
            resultCode = "AcceptanceCouldNotBeObserved";
            reason = "Docker accepted the operation, but the observed Service does not contain its operation identity.";
            return true;
        }

        resultCode = "AcceptedRuntimeMismatch";
        reason = "Docker accepted the operation, but the observed Service configuration does not match the requested configuration.";
        return true;
    }

    internal static bool IsDeleteSatisfied(
        SwarmServiceOperation operation,
        SwarmServiceProjection? live) =>
        operation.Kind == SwarmServiceOperationKind.Delete
        && operation.State is (SwarmServiceOperationState.PendingAcceptance
            or SwarmServiceOperationState.Accepted
            or SwarmServiceOperationState.OutcomeUnknown)
        && live is null;

    internal static bool TryGetRolloutFailure(
        SwarmServiceProjection service,
        IReadOnlyList<SwarmTaskProjection> tasks,
        out string resultCode,
        out string reason)
    {
        var paused = service.UpdateState.Equals("Paused", StringComparison.OrdinalIgnoreCase)
            || service.UpdateState.Equals("RollbackPaused", StringComparison.OrdinalIgnoreCase)
            || service.UpdateState.Equals("RollbackCompleted", StringComparison.OrdinalIgnoreCase)
            || service.UpdateState.Equals("rollback_paused", StringComparison.OrdinalIgnoreCase)
            || service.UpdateState.Equals("rollback_completed", StringComparison.OrdinalIgnoreCase);
        if (paused)
        {
            resultCode = "RolloutPaused";
            reason = GetTaskFailure(service.DockerServiceId, tasks)
                ?? service.UpdateMessage
                ?? $"Docker paused the Service rollout in state '{service.UpdateState}'.";
            return true;
        }

        if (IsRolloutCompleteState(service.UpdateState)
            && service.RunningTaskCount < service.DesiredTaskCount
            && GetTaskFailure(service.DockerServiceId, tasks) is { } taskFailure)
        {
            resultCode = "TaskFailed";
            reason = taskFailure;
            return true;
        }

        resultCode = string.Empty;
        reason = string.Empty;
        return false;
    }

    internal static bool IsRolloutComplete(SwarmServiceProjection service) =>
        IsRolloutCompleteState(service.UpdateState)
        && service.RunningTaskCount >= service.DesiredTaskCount;

    private static bool IsRolloutCompleteState(string state) =>
        state.Equals("None", StringComparison.OrdinalIgnoreCase)
        || state.Equals("Completed", StringComparison.OrdinalIgnoreCase);

    private static string? GetTaskFailure(
        string dockerServiceId,
        IReadOnlyList<SwarmTaskProjection> tasks)
    {
        foreach (var task in tasks)
        {
            if (!task.IsStale
                && task.DockerServiceId.Equals(dockerServiceId, StringComparison.Ordinal)
                && !string.IsNullOrWhiteSpace(task.Error))
                return task.Error;
        }

        return null;
    }

    private static bool IsTerminal(SwarmServiceOperationState state) => state is
        SwarmServiceOperationState.Canceled
        or SwarmServiceOperationState.Rejected
        or SwarmServiceOperationState.NotAccepted
        or SwarmServiceOperationState.Completed
        or SwarmServiceOperationState.OwnershipConflict;

    private static ActivityEvent CreateFailureActivity(
        SwarmService service,
        SwarmServiceOperation operation,
        string reason) =>
        new(
            service.PlatformId,
            service.Id,
            operation.ActorId ?? Constants.SystemId,
            service.Name,
            ActivityEventType.SwarmServiceOperationFailed,
            ActivityStatus.Failure,
            new SwarmServiceOperationFailed(operation.Id, operation.Kind, reason));

    private static ActivityEvent CreateSuccessActivity(
        SwarmService service,
        SwarmServiceOperation operation)
    {
        var (eventType, info) = operation.Kind switch
        {
            SwarmServiceOperationKind.Scale => (
                ActivityEventType.SwarmServiceScaled,
                (ActivityEventInfo)new SwarmServiceScaled(
                    operation.Id,
                    service.Spec.Replicas ?? 0,
                    operation.Warnings ?? [])),
            SwarmServiceOperationKind.ForceUpdate => (
                ActivityEventType.SwarmServiceForceUpdated,
                new SwarmServiceForceUpdated(operation.Id, operation.Warnings ?? [])),
            _ => (
                ActivityEventType.SwarmServiceApplied,
                new SwarmServiceApplied(operation.Id, operation.Warnings ?? []))
        };
        return new ActivityEvent(
            service.PlatformId,
            service.Id,
            operation.ActorId ?? Constants.SystemId,
            service.Name,
            eventType,
            ActivityStatus.Success,
            info);
    }

    private static string? ExtractDigest(string image)
    {
        var separator = image.LastIndexOf('@');
        return separator >= 0 && separator < image.Length - 1
            ? image[(separator + 1)..]
            : null;
    }
}

internal sealed record ManagedSwarmServiceChange(SwarmService Service, string Action);

internal sealed class SwarmInventoryUpdatedNotificationWorkItem(
    IApplicationHubDispatcher hubDispatcher,
    Guid platformId,
    SwarmProjectionSnapshot snapshot,
    IReadOnlyList<ManagedSwarmServiceChange> managedChanges) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var notifications = new Task[managedChanges.Count + 1];
        notifications[0] = hubDispatcher.SendSwarmInventory(platformId, snapshot, cancellationToken);
        for (var index = 0; index < managedChanges.Count; index++)
        {
            var change = managedChanges[index];
            notifications[index + 1] = hubDispatcher.SendSwarmServiceInfo(change.Service, change.Action);
        }
        return Task.WhenAll(notifications);
    }
}
