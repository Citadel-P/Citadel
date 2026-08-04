using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
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
                    new ListSwarmNodesCommand(platform.Address, SwarmInventoryLimits.MaximumItems, IncludeTaskCounts: true),
                    timeout.Token);
                var servicesTask = connector.ListServicesAsync(new ListSwarmServicesCommand(platform.Address), timeout.Token);
                var tasksTask = connector.ListTasksAsync(new ListSwarmTasksCommand(platform.Address), timeout.Token);
                var networksTask = connector.ListNetworksAsync(new ListSwarmNetworksCommand(platform.Address), timeout.Token);
                var secretsTask = connector.ListSecretsAsync(new ListSwarmSecretsCommand(platform.Address), timeout.Token);
                var configsTask = connector.ListConfigsAsync(new ListSwarmConfigsCommand(platform.Address), timeout.Token);
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
        var workItem = new PersistSwarmSnapshotWorkItem(platformId, snapshot);
        await dbQueue.EnqueueAndWaitAsync(workItem, cancellationToken);
        await TryNotifyAsync(platformId, workItem.Current, cancellationToken);
    }

    private async Task TryNotifyAsync(
        Guid platformId,
        SwarmProjectionSnapshot snapshot,
        CancellationToken cancellationToken)
    {
        try
        {
            using var enqueueTimeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            enqueueTimeout.CancelAfter(NotificationEnqueueTimeout);
            await notificationQueue.EnqueueAsync(
                new SwarmInventoryUpdatedNotificationWorkItem(hubDispatcher, platformId, snapshot),
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
    SwarmProjectionSnapshot? snapshot) : IDbWorkItem
{
    public SwarmProjectionSnapshot Current { get; private set; } = null!;

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
            await unitOfWork.Swarm.ReplaceAsync(platformId, snapshot, cancellationToken);
            Current = snapshot;
        }

        await unitOfWork.CommitAsync(cancellationToken);
    }
}

internal sealed class SwarmInventoryUpdatedNotificationWorkItem(
    IApplicationHubDispatcher hubDispatcher,
    Guid platformId,
    SwarmProjectionSnapshot snapshot) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken) =>
        hubDispatcher.SendSwarmInventory(platformId, snapshot, cancellationToken);
}
