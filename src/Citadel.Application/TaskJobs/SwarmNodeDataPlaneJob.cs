using Application.Configs;
using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.ObjectPool;
using Microsoft.Extensions.Options;
using System.Collections.Concurrent;
using System.Collections.Immutable;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class SwarmNodeDataPlaneJob(
    IServiceScopeFactory scopeFactory,
    ISwarmNodeRuntimeConnector connector,
    IDbWorkQueue dbQueue,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IDockerDaemonStreamManager dockerDaemonStreamManager,
    IContainerEventBroadcaster containerEventBroadcaster,
    ChannelWriter<UnmanagedContainerAlertRequest> unmanagedContainerAlertWriter,
    ChannelWriter<ContainersStatBatch> statsWriter,
    IOptions<JobConfiguration> options,
    ILogger<SwarmNodeDataPlaneJob> logger)
    : BackgroundService, ISwarmNodeDataPlaneCoordinator
{
    private const int HistoricalTaskPruneBatchSize = 100;
    private const int MaximumConcurrentReconciliations = 16;
    private const int MaximumConcurrentPlatformReconciliations = 4;
    private const int PlatformGateCount = 64;
    private static readonly TimeSpan ReconciliationInterval = TimeSpan.FromMinutes(30);
    private static readonly TimeSpan StreamReconnectDelay = TimeSpan.FromSeconds(10);
    private readonly int _statsIntervalMs = options.Value.MonitoringInterval * 1000;
    private readonly Channel<SessionChange> _sessionChanges = Channel.CreateBounded<SessionChange>(
        new BoundedChannelOptions(256)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });
    private readonly ConcurrentDictionary<NodeKey, NodeMonitor> _monitors = new();
    private readonly ConcurrentDictionary<Guid, string> _managerNodes = new();
    private readonly SemaphoreSlim _reconciliationSlots = new(
        MaximumConcurrentReconciliations,
        MaximumConcurrentReconciliations);
    private readonly SemaphoreSlim[] _platformReconciliationSlots = CreatePlatformGates();
    private readonly ObjectPool<List<ContainerStat>> _statsPool =
        new DefaultObjectPool<List<ContainerStat>>(new ContainerStatsListPooledObjectPolicy());

    public bool HandlesManagerLocalResources(Guid platformId) => _managerNodes.ContainsKey(platformId);

    public async ValueTask NotifyManagerConnectedAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var platform = await LoadPlatformAsync(platformId, cancellationToken);
        if (platform?.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor
            || string.IsNullOrWhiteSpace(descriptor.NodeID))
        {
            return;
        }

        _managerNodes[platformId] = descriptor.NodeID;
        await _sessionChanges.Writer.WriteAsync(
            new SessionChange(
                new NodeKey(platformId, descriptor.NodeID),
                "manager-connector",
                Connected: true,
                OwnsStreams: false),
            cancellationToken);
    }

    public ValueTask NotifyManagerDisconnectedAsync(
        Guid platformId,
        CancellationToken cancellationToken)
    {
        if (!_managerNodes.TryRemove(platformId, out var dockerNodeId))
            return ValueTask.CompletedTask;

        return _sessionChanges.Writer.WriteAsync(
            new SessionChange(
                new NodeKey(platformId, dockerNodeId),
                "manager-connector",
                Connected: false,
                OwnsStreams: false),
            cancellationToken);
    }

    public ValueTask NotifyManagerDaemonEventAsync(
        Guid platformId,
        DaemonEventInfo daemonEvent,
        CancellationToken cancellationToken)
    {
        if (_managerNodes.TryGetValue(platformId, out var dockerNodeId)
            && _monitors.TryGetValue(new NodeKey(platformId, dockerNodeId), out var monitor)
            && daemonEvent.Scope != DaemonEventScope.Swarm)
        {
            monitor.ReconcileSignals.Writer.TryWrite(true);
        }

        return ValueTask.CompletedTask;
    }

    public ValueTask NotifyConnectedAsync(
        Guid platformId,
        string dockerNodeId,
        string sessionId,
        CancellationToken cancellationToken)
        => _sessionChanges.Writer.WriteAsync(
            new SessionChange(new NodeKey(platformId, dockerNodeId), sessionId, Connected: true, OwnsStreams: true),
            cancellationToken);

    public ValueTask NotifyDisconnectedAsync(
        Guid platformId,
        string dockerNodeId,
        string sessionId,
        CancellationToken cancellationToken)
        => _sessionChanges.Writer.WriteAsync(
            new SessionChange(new NodeKey(platformId, dockerNodeId), sessionId, Connected: false, OwnsStreams: true),
            cancellationToken);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            await foreach (var change in _sessionChanges.Reader.ReadAllAsync(stoppingToken))
            {
                if (change.Connected)
                    StartMonitor(change, stoppingToken);
                else
                    StopMonitor(change);
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
        {
        }
        finally
        {
            var monitors = _monitors.Values.ToArray();
            foreach (var monitor in monitors)
                monitor.Cancel();
            await Task.WhenAll(monitors.Select(static monitor => monitor.Task));
        }
    }

    private void StartMonitor(SessionChange change, CancellationToken stoppingToken)
    {
        var monitor = new NodeMonitor(
            change.Key,
            change.SessionId,
            change.OwnsStreams,
            CancellationTokenSource.CreateLinkedTokenSource(stoppingToken));

        while (true)
        {
            if (!_monitors.TryGetValue(change.Key, out var previous))
            {
                if (!_monitors.TryAdd(change.Key, monitor))
                    continue;
                break;
            }

            if (string.Equals(previous.SessionId, change.SessionId, StringComparison.Ordinal))
            {
                monitor.Dispose();
                return;
            }

            if (!_monitors.TryUpdate(change.Key, monitor, previous))
                continue;

            previous.Cancel();
            break;
        }

        monitor.Task = MonitorNodeAsync(monitor);
    }

    private void StopMonitor(SessionChange change)
    {
        if (_monitors.TryGetValue(change.Key, out var monitor)
            && string.Equals(monitor.SessionId, change.SessionId, StringComparison.Ordinal)
            && ((ICollection<KeyValuePair<NodeKey, NodeMonitor>>)_monitors)
                .Remove(new KeyValuePair<NodeKey, NodeMonitor>(change.Key, monitor)))
        {
            monitor.Cancel();
        }
    }

    private async Task MonitorNodeAsync(NodeMonitor monitor)
    {
        try
        {
            var platform = await LoadPlatformAsync(monitor.Key.PlatformId, monitor.Token);
            if (platform?.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
                return;

            if (monitor.OwnsStreams)
            {
                await Task.WhenAll(
                    RunReconciliationLoopAsync(platform, monitor),
                    RunEventLoopAsync(platform, monitor),
                    RunStatsLoopAsync(platform, monitor));
            }
            else
            {
                await RunReconciliationLoopAsync(platform, monitor);
            }
        }
        catch (OperationCanceledException) when (monitor.Token.IsCancellationRequested)
        {
        }
        catch (Exception exception)
        {
            logger.LogError(
                exception,
                "Swarm Node data-plane monitor failed for Platform {PlatformId}, Node {DockerNodeId}.",
                monitor.Key.PlatformId,
                monitor.Key.DockerNodeId);
        }
        finally
        {
            if (!_monitors.TryGetValue(monitor.Key, out var replacement)
                || ReferenceEquals(replacement, monitor))
            {
                using var staleTimeout = new CancellationTokenSource(TimeSpan.FromSeconds(10));
                await MarkStaleAsync(
                    monitor.Key,
                    "Node Agent disconnected.",
                    staleTimeout.Token);
            }
            ((ICollection<KeyValuePair<NodeKey, NodeMonitor>>)_monitors)
                .Remove(new KeyValuePair<NodeKey, NodeMonitor>(monitor.Key, monitor));
            monitor.Dispose();
        }
    }

    private async Task RunReconciliationLoopAsync(Platform platform, NodeMonitor monitor)
    {
        while (!monitor.Token.IsCancellationRequested)
        {
            await ReconcileAsync(platform, monitor);

            var signaled = false;
            using (var wait = CancellationTokenSource.CreateLinkedTokenSource(monitor.Token))
            {
                wait.CancelAfter(ReconciliationInterval);
                try
                {
                    signaled = await monitor.ReconcileSignals.Reader.WaitToReadAsync(wait.Token);
                }
                catch (OperationCanceledException) when (!monitor.Token.IsCancellationRequested)
                {
                    // Periodic safety pass.
                }
            }

            if (signaled)
            {
                await Task.Delay(TimeSpan.FromMilliseconds(250), monitor.Token);
                while (monitor.ReconcileSignals.Reader.TryRead(out _)) { }
            }
        }
    }

    private async Task ReconcileAsync(Platform platform, NodeMonitor monitor)
    {
        var platformGate = GetPlatformGate(platform.Id);
        await platformGate.WaitAsync(monitor.Token);
        try
        {
            await _reconciliationSlots.WaitAsync(monitor.Token);
            try
            {
                var startedAt = DateTimeOffset.UtcNow;
                var start = new StartSwarmNodeReconciliationWorkItem(monitor.Key, startedAt);
                await dbQueue.EnqueueAndWaitAsync(start, monitor.Token);
                var containersTask = connector.ListContainersAsync(
                    platform,
                    monitor.Key.DockerNodeId,
                    monitor.Token);
                var imagesTask = connector.ListImagesAsync(
                    platform,
                    monitor.Key.DockerNodeId,
                    monitor.Token);
                var volumesTask = connector.ListVolumesAsync(
                    platform,
                    monitor.Key.DockerNodeId,
                    monitor.Token);
                var networksTask = connector.ListNetworksAsync(
                    platform,
                    monitor.Key.DockerNodeId,
                    monitor.Token);
                await Task.WhenAll(containersTask, imagesTask, volumesTask, networksTask);

                var containersResult = await containersTask;
                var imagesResult = await imagesTask;
                var volumesResult = await volumesTask;
                var networksResult = await networksTask;
                var containersSucceeded = containersResult.IsSuccess(out var containers, out var containersError);
                var imagesSucceeded = imagesResult.IsSuccess(out var images, out var imagesError);
                var volumesSucceeded = volumesResult.IsSuccess(out var volumes, out var volumesError);
                var networksSucceeded = networksResult.IsSuccess(out var networks, out var networksError);
                if (!containersSucceeded || !imagesSucceeded || !volumesSucceeded || !networksSucceeded)
                {
                    await MarkStaleAsync(
                        monitor.Key,
                        containersError?.Message
                        ?? imagesError?.Message
                        ?? volumesError?.Message
                        ?? networksError?.Message
                        ?? "Node inventory could not be refreshed.",
                        monitor.Token);
                    return;
                }

                await PruneHistoricalTaskContainersAsync(platform, monitor, containers!.Values);

                var observedAt = DateTimeOffset.UtcNow;
                var imageProjections = images!
                    .Select(image => SwarmNodeImageProjection.FromObservation(
                        platform.Id,
                        monitor.Key.DockerNodeId,
                        image,
                        observedAt))
                    .ToArray();
                var volumeProjections = volumes!
                    .Where(static volume => string.Equals(volume.Scope, "local", StringComparison.OrdinalIgnoreCase))
                    .Select(volume => SwarmNodeVolumeProjection.FromObservation(
                        platform.Id,
                        monitor.Key.DockerNodeId,
                        volume,
                        observedAt))
                    .ToArray();
                var networkProjections = networks!
                    .Where(static network => !string.Equals(network.Scope, "swarm", StringComparison.OrdinalIgnoreCase))
                    .Select(network => SwarmNodeNetworkProjection.FromObservation(
                        platform.Id,
                        monitor.Key.DockerNodeId,
                        network,
                        observedAt))
                    .ToArray();

                await dbQueue.EnqueueAndWaitAsync(
                    new ReconcileSwarmNodeContainersWorkItem(
                        monitor.Key,
                        containers,
                        startedAt.ToUnixTimeSeconds(),
                        start.Generation,
                        monitor,
                        notificationQueue,
                        platformContainerCache,
                        containerStreamManager,
                        dockerDaemonStreamManager,
                        imageProjections,
                        volumeProjections,
                        networkProjections),
                    monitor.Token);
            }
            finally
            {
                _reconciliationSlots.Release();
            }
        }
        catch (OperationCanceledException) when (monitor.Token.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogWarning(
                exception,
                "Failed to reconcile Containers for Platform {PlatformId}, Node {DockerNodeId}.",
                monitor.Key.PlatformId,
                monitor.Key.DockerNodeId);
            await MarkStaleAsync(monitor.Key, "Node inventory could not be refreshed.", monitor.Token);
        }
        finally
        {
            platformGate.Release();
        }
    }

    private async Task PruneHistoricalTaskContainersAsync(
        Platform platform,
        NodeMonitor monitor,
        IEnumerable<DockerContainer> containers)
    {
        if (!platform.PruneHistoricalSwarmTaskContainers)
            return;

        var historicalIds = containers
            .Where(static container => container.IsHistoricalSwarmTask())
            .Select(static container => container.Id)
            .Distinct(StringComparer.Ordinal)
            .Take(HistoricalTaskPruneBatchSize)
            .ToArray();
        if (historicalIds.Length == 0)
            return;

        var result = await connector.DeleteContainersAsync(
            platform,
            monitor.Key.DockerNodeId,
            historicalIds,
            volumes: false,
            force: false,
            link: false,
            monitor.Token);
        if (result.IsFailure(out var error))
        {
            logger.LogWarning(
                "Failed to prune {Count} historical Swarm Task containers for Platform {PlatformId}, Node {DockerNodeId}: {Error}",
                historicalIds.Length,
                monitor.Key.PlatformId,
                monitor.Key.DockerNodeId,
                error!.Message);
        }
    }

    private async Task RunEventLoopAsync(Platform platform, NodeMonitor monitor)
    {
        while (!monitor.Token.IsCancellationRequested)
        {
            try
            {
                await dbQueue.EnqueueAsync(
                    new UpdateSwarmNodeEventStateWorkItem(monitor.Key, Connected: true),
                    monitor.Token);
                await foreach (var daemonEvent in connector.StreamDaemonEventsAsync(
                                   platform,
                                   monitor.Key.DockerNodeId,
                                   monitor.Token))
                {
                    if (daemonEvent is DaemonContainerEventInfo containerEvent)
                    {
                        var apply = new ApplySwarmNodeContainerEventWorkItem(
                            monitor.Key,
                            containerEvent,
                            monitor,
                            notificationQueue,
                            dockerDaemonStreamManager,
                            containerEventBroadcaster,
                            unmanagedContainerAlertWriter,
                            platformContainerCache);
                        await dbQueue.EnqueueAndWaitAsync(apply, monitor.Token);
                        if (!apply.Applied)
                            monitor.ReconcileSignals.Writer.TryWrite(true);
                    }
                    else if (daemonEvent is DaemonImageEventInfo
                             or DaemonVolumeEventInfo
                             or DaemonNetworkEventInfo)
                    {
                        monitor.ReconcileSignals.Writer.TryWrite(true);
                    }
                }
            }
            catch (OperationCanceledException) when (monitor.Token.IsCancellationRequested)
            {
                break;
            }
            catch (Exception exception)
            {
                logger.LogWarning(
                    exception,
                    "Node event stream failed for Platform {PlatformId}, Node {DockerNodeId}.",
                    monitor.Key.PlatformId,
                    monitor.Key.DockerNodeId);
            }

            if (!monitor.Token.IsCancellationRequested)
            {
                await dbQueue.EnqueueAsync(
                    new UpdateSwarmNodeEventStateWorkItem(monitor.Key, Connected: false),
                    monitor.Token);
                monitor.ReconcileSignals.Writer.TryWrite(true);
            }

            await Task.Delay(StreamReconnectDelay, monitor.Token);
        }
    }

    private async Task RunStatsLoopAsync(Platform platform, NodeMonitor monitor)
    {
        while (!monitor.Token.IsCancellationRequested)
        {
            try
            {
                await foreach (var snapshot in connector.StreamContainersStatsAsync(
                                   platform,
                                   monitor.Key.DockerNodeId,
                                   _statsIntervalMs,
                                   monitor.Token))
                {
                    var ids = monitor.ContainerIds;
                    var stats = _statsPool.Get();
                    var observedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                    foreach (var (dockerContainerId, value) in snapshot)
                    {
                        if (ids.TryGetValue(dockerContainerId, out var containerId))
                            stats.Add(value.Map(containerId, observedAt));
                    }

                    if (stats.Count == 0)
                    {
                        ReturnStatsList(stats);
                        continue;
                    }

                    var batch = new ContainersStatBatch(
                        platform.Id,
                        stats,
                        ReturnStatsList,
                        monitor.Key.DockerNodeId);
                    var ownershipTransferred = false;
                    try
                    {
                        await statsWriter.WriteAsync(batch, monitor.Token);
                        ownershipTransferred = true;
                    }
                    finally
                    {
                        if (!ownershipTransferred)
                            batch.Release();
                    }
                }
            }
            catch (OperationCanceledException) when (monitor.Token.IsCancellationRequested)
            {
                break;
            }
            catch (Exception exception)
            {
                logger.LogWarning(
                    exception,
                    "Node statistics stream failed for Platform {PlatformId}, Node {DockerNodeId}.",
                    monitor.Key.PlatformId,
                    monitor.Key.DockerNodeId);
            }

            await Task.Delay(StreamReconnectDelay, monitor.Token);
        }
    }

    private async Task<Platform?> LoadPlatformAsync(Guid platformId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
    }

    private async Task MarkStaleAsync(NodeKey key, string reason, CancellationToken cancellationToken)
    {
        try
        {
            await dbQueue.EnqueueAndWaitAsync(
                new MarkSwarmNodeContainersStaleWorkItem(
                    key,
                    reason,
                    notificationQueue,
                    containerStreamManager,
                    dockerDaemonStreamManager),
                cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (Exception exception)
        {
            logger.LogWarning(
                exception,
                "Failed to mark Container projections stale for Platform {PlatformId}, Node {DockerNodeId}.",
                key.PlatformId,
                key.DockerNodeId);
        }
    }

    private SemaphoreSlim GetPlatformGate(Guid platformId)
    {
        var hash = (uint)platformId.GetHashCode();
        return _platformReconciliationSlots[hash % (uint)PlatformGateCount];
    }

    private void ReturnStatsList(List<ContainerStat> stats) => _statsPool.Return(stats);

    private static SemaphoreSlim[] CreatePlatformGates()
    {
        var gates = new SemaphoreSlim[PlatformGateCount];
        for (var index = 0; index < gates.Length; index++)
        {
            gates[index] = new SemaphoreSlim(
                MaximumConcurrentPlatformReconciliations,
                MaximumConcurrentPlatformReconciliations);
        }

        return gates;
    }

    internal readonly record struct NodeKey(Guid PlatformId, string DockerNodeId);
    private readonly record struct SessionChange(
        NodeKey Key,
        string SessionId,
        bool Connected,
        bool OwnsStreams);

    internal sealed class NodeMonitor(
        NodeKey key,
        string sessionId,
        bool ownsStreams,
        CancellationTokenSource cancellation) : IDisposable
    {
        private ImmutableDictionary<string, Guid> _containerIds = ImmutableDictionary<string, Guid>.Empty;

        public NodeKey Key { get; } = key;
        public string SessionId { get; } = sessionId;
        public bool OwnsStreams { get; } = ownsStreams;
        public CancellationToken Token => cancellation.Token;
        public Task Task { get; set; } = Task.CompletedTask;
        public Channel<bool> ReconcileSignals { get; } = Channel.CreateBounded<bool>(
            new BoundedChannelOptions(1)
            {
                FullMode = BoundedChannelFullMode.DropOldest,
                SingleReader = true,
                SingleWriter = true
            });
        public ImmutableDictionary<string, Guid> ContainerIds => Volatile.Read(ref _containerIds);

        public void SetContainerIds(ImmutableDictionary<string, Guid> value)
            => Volatile.Write(ref _containerIds, value);

        public void UpsertContainerId(string dockerContainerId, Guid containerId)
            => Volatile.Write(
                ref _containerIds,
                ContainerIds.SetItem(dockerContainerId, containerId));

        public void RemoveContainerId(string dockerContainerId)
            => Volatile.Write(
                ref _containerIds,
                ContainerIds.Remove(dockerContainerId));

        public void Cancel()
        {
            try { cancellation.Cancel(); } catch { }
        }

        public void Dispose() => cancellation.Dispose();
    }

    private sealed class ApplySwarmNodeContainerEventWorkItem(
        NodeKey key,
        DaemonContainerEventInfo eventInfo,
        NodeMonitor monitor,
        INotificationQueue notificationQueue,
        IDockerDaemonStreamManager dockerDaemonStreamManager,
        IContainerEventBroadcaster containerEventBroadcaster,
        ChannelWriter<UnmanagedContainerAlertRequest> unmanagedContainerAlertWriter,
        IPlatformContainerCache platformContainerCache) : IDbWorkItem
    {
        public bool Applied { get; private set; }

        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var existing = await uow.Containers.GetByRuntimeIdentityAsync(
                key.PlatformId,
                key.DockerNodeId,
                eventInfo.ContainerId,
                cancellationToken);

            if (string.Equals(eventInfo.Action, "destroy", StringComparison.OrdinalIgnoreCase)
                || eventInfo.Container?.IsHistoricalSwarmTask() == true)
            {
                if (existing is not null)
                {
                    await uow.Containers.DeleteAsync([existing.Id], cancellationToken);
                    await uow.CommitAsync(cancellationToken);
                    platformContainerCache.TryRemoveContainer(key.PlatformId, existing.DockerContainerId);
                    monitor.RemoveContainerId(existing.DockerContainerId);

                    var removalEvent = string.Equals(eventInfo.Action, "destroy", StringComparison.OrdinalIgnoreCase)
                        ? eventInfo
                        : eventInfo with { Action = "destroy" };
                    await notificationQueue.EnqueueAsync(
                        new ContainerNotificationWorkItem(
                            existing,
                            removalEvent,
                            dockerDaemonStreamManager,
                            containerEventBroadcaster),
                        cancellationToken);
                }

                Applied = true;
                return;
            }

            var fresh = eventInfo.Container;
            if (fresh is null)
                return;

            var observedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var container = existing ?? fresh.Map(
                key.PlatformId,
                imageId: null,
                key.DockerNodeId,
                observedAt);

            if (existing is not null)
            {
                container.PartialUpdate(
                    name: fresh.Name,
                    dockerImageId: fresh.ImageId,
                    state: fresh.State,
                    dockerStack: fresh.Stack,
                    created: fresh.Created,
                    ports: fresh.Ports,
                    stackId: container.StackId ?? fresh.StackId,
                    isSystem: fresh.IsSystem,
                    systemRole: fresh.SystemRole,
                    hasCitadelOwnershipLabels: fresh.HasCitadelOwnershipLabels,
                    isSwarmTask: fresh.IsSwarmTask);
                container.ObserveOnNode(key.DockerNodeId, observedAt);
            }

            await uow.Containers.BulkUpsertAsync([container], cancellationToken);
            await uow.CommitAsync(cancellationToken);

            platformContainerCache.TryAddContainer(key.PlatformId, container.DockerContainerId, container.Id);
            monitor.UpsertContainerId(container.DockerContainerId, container.Id);
            await notificationQueue.EnqueueAsync(
                new ContainerNotificationWorkItem(
                    container,
                    eventInfo,
                    dockerDaemonStreamManager,
                    containerEventBroadcaster),
                cancellationToken);

            if (existing is null
                && !container.IsSystem
                && !container.HasCitadelOwnershipLabels
                && !container.IsSwarmTask)
            {
                await unmanagedContainerAlertWriter.WriteAsync(
                    new UnmanagedContainerAlertRequest(key.PlatformId, container.DockerContainerId),
                    cancellationToken);
            }

            Applied = true;
        }
    }

    internal sealed class ReconcileSwarmNodeContainersWorkItem(
        NodeKey key,
        IReadOnlyDictionary<string, DockerContainer> freshContainers,
        long snapshotStartedAt,
        long generation,
        NodeMonitor monitor,
        INotificationQueue notificationQueue,
        IPlatformContainerCache platformContainerCache,
        IContainerStreamManager containerStreamManager,
        IDockerDaemonStreamManager dockerDaemonStreamManager,
        IReadOnlyList<SwarmNodeImageProjection>? images = null,
        IReadOnlyList<SwarmNodeVolumeProjection>? volumes = null,
        IReadOnlyList<SwarmNodeNetworkProjection>? networks = null) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var runtimeState = await uow.Swarm.GetNodeRuntimeStateAsync(
                key.PlatformId,
                key.DockerNodeId,
                cancellationToken);
            if (runtimeState is null || runtimeState.ReconciliationGeneration != generation)
                return;

            var observedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var existing = (await uow.Containers.GetByPlatformAndNodeIdAsync(
                    key.PlatformId,
                    key.DockerNodeId,
                    cancellationToken))
                .ToDictionary(container => container.DockerContainerId, StringComparer.OrdinalIgnoreCase);
            var current = freshContainers.Values
                .Where(static container => !container.IsHistoricalSwarmTask())
                .ToArray();
            var upserts = new List<Container>(current.Length);

            foreach (var fresh in current)
            {
                if (existing.TryGetValue(fresh.Id, out var container))
                {
                    if (container.ProjectionObservedAt >= snapshotStartedAt)
                    {
                        upserts.Add(container);
                        continue;
                    }

                    container.PartialUpdate(
                        name: fresh.Name,
                        dockerImageId: fresh.ImageId,
                        state: fresh.State,
                        dockerStack: fresh.Stack,
                        created: fresh.Created,
                        ports: fresh.Ports,
                        stackId: container.StackId ?? fresh.StackId,
                        isSystem: fresh.IsSystem,
                        systemRole: fresh.SystemRole,
                        hasCitadelOwnershipLabels: fresh.HasCitadelOwnershipLabels,
                        isSwarmTask: fresh.IsSwarmTask);
                    container.ObserveOnNode(key.DockerNodeId, observedAt);
                }
                else
                {
                    container = fresh.Map(
                        key.PlatformId,
                        imageId: null,
                        key.DockerNodeId,
                        observedAt);
                }

                upserts.Add(container);
            }

            if (upserts.Count > 0)
                await uow.Containers.BulkUpsertAsync(upserts, cancellationToken);

            var currentIds = current
                .Select(static container => container.Id)
                .ToHashSet(StringComparer.OrdinalIgnoreCase);
            var removed = existing.Values
                .Where(container => !currentIds.Contains(container.DockerContainerId)
                                    && container.ProjectionObservedAt < snapshotStartedAt)
                .ToArray();
            if (removed.Length > 0)
                await uow.Containers.DeleteAsync(removed.Select(static container => container.Id), cancellationToken);

            await uow.Swarm.ReplaceNodeLocalResourcesAsync(
                key.PlatformId,
                key.DockerNodeId,
                images ?? [],
                volumes ?? [],
                networks ?? [],
                DateTimeOffset.FromUnixTimeSeconds(snapshotStartedAt),
                cancellationToken);

            var completedAt = DateTimeOffset.UtcNow;
            await uow.Swarm.UpsertNodeRuntimeStateAsync(runtimeState with
            {
                ReconciliationCompletedAt = completedAt,
                LastSuccessfulReconciliationAt = completedAt,
                IsStale = false,
                StaleSince = null,
                StaleReason = null
            }, cancellationToken);

            await uow.CommitAsync(cancellationToken);

            foreach (var container in removed)
                platformContainerCache.TryRemoveContainer(key.PlatformId, container.DockerContainerId);
            foreach (var container in upserts)
                platformContainerCache.TryAddContainer(key.PlatformId, container.DockerContainerId, container.Id);

            var retained = existing.Values
                .Where(container => !currentIds.Contains(container.DockerContainerId)
                                    && container.ProjectionObservedAt >= snapshotStartedAt)
                .ToArray();
            monitor.SetContainerIds(upserts.Concat(retained).ToImmutableDictionary(
                static container => container.DockerContainerId,
                static container => container.Id,
                StringComparer.OrdinalIgnoreCase));

            if (containerStreamManager.HasStatsSubscribers(key.PlatformId))
            {
                var all = (await uow.Containers.GetContainersInfoAsync(key.PlatformId, cancellationToken))?.ToArray() ?? [];
                await notificationQueue.EnqueueAsync(
                    new SendContainersInfoNotificationWorkItem(containerStreamManager, all, key.PlatformId),
                    cancellationToken);
            }

            await notificationQueue.EnqueueAsync(
                new SwarmNodeLocalResourcesNotificationWorkItem(
                    dockerDaemonStreamManager,
                    await LoadNodeLocalResourceSnapshotAsync(uow, key.PlatformId, cancellationToken)),
                cancellationToken);
        }
    }

    private sealed class MarkSwarmNodeContainersStaleWorkItem(
        NodeKey key,
        string reason,
        INotificationQueue notificationQueue,
        IContainerStreamManager containerStreamManager,
        IDockerDaemonStreamManager dockerDaemonStreamManager) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var now = DateTimeOffset.UtcNow;
            await uow.Containers.MarkNodeProjectionStaleAsync(
                key.PlatformId,
                key.DockerNodeId,
                reason,
                now.ToUnixTimeSeconds(),
                cancellationToken);
            await uow.Swarm.MarkNodeLocalResourcesStaleAsync(
                key.PlatformId,
                key.DockerNodeId,
                cancellationToken);
            var state = await uow.Swarm.GetNodeRuntimeStateAsync(key.PlatformId, key.DockerNodeId, cancellationToken)
                        ?? CreateInitialRuntimeState(key, now);
            await uow.Swarm.UpsertNodeRuntimeStateAsync(state with
            {
                IsStale = true,
                StaleSince = state.StaleSince ?? now,
                StaleReason = reason
            }, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            if (containerStreamManager.HasStatsSubscribers(key.PlatformId))
            {
                var all = (await uow.Containers.GetContainersInfoAsync(key.PlatformId, cancellationToken))?.ToArray() ?? [];
                await notificationQueue.EnqueueAsync(
                    new SendContainersInfoNotificationWorkItem(containerStreamManager, all, key.PlatformId),
                    cancellationToken);
            }


            await notificationQueue.EnqueueAsync(
                new SwarmNodeLocalResourcesNotificationWorkItem(
                    dockerDaemonStreamManager,
                    await LoadNodeLocalResourceSnapshotAsync(uow, key.PlatformId, cancellationToken)),
                cancellationToken);
        }
    }

    private static async Task<SwarmNodeLocalResourceSnapshot> LoadNodeLocalResourceSnapshotAsync(
        IUnitOfWork uow,
        Guid platformId,
        CancellationToken cancellationToken) => new(
        platformId,
        await uow.Swarm.GetNodeImagesAsync(platformId, cancellationToken),
        await uow.Swarm.GetNodeVolumesAsync(platformId, cancellationToken),
        await uow.Swarm.GetNodeNetworksAsync(platformId, cancellationToken));

    private sealed class StartSwarmNodeReconciliationWorkItem(NodeKey key, DateTimeOffset startedAt) : IDbWorkItem
    {
        public long Generation { get; private set; }

        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var current = await uow.Swarm.GetNodeRuntimeStateAsync(key.PlatformId, key.DockerNodeId, cancellationToken)
                          ?? CreateInitialRuntimeState(key, startedAt);
            var binding = await uow.EdgeAgents.GetNodeBindingAsync(key.PlatformId, key.DockerNodeId, cancellationToken);
            var node = await uow.Swarm.GetNodeAsync(key.PlatformId, key.DockerNodeId, cancellationToken);
            Generation = checked(current.ReconciliationGeneration + 1);
            await uow.Swarm.UpsertNodeRuntimeStateAsync(current with
            {
                ReconciliationGeneration = Generation,
                ReconciliationStartedAt = startedAt,
                AgentVersion = binding?.LastSeenVersion,
                DockerVersion = node?.EngineVersion
            }, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }
    }

    private sealed class UpdateSwarmNodeEventStateWorkItem(NodeKey key, bool Connected) : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var now = DateTimeOffset.UtcNow;
            var current = await uow.Swarm.GetNodeRuntimeStateAsync(key.PlatformId, key.DockerNodeId, cancellationToken)
                          ?? CreateInitialRuntimeState(key, now);
            await uow.Swarm.UpsertNodeRuntimeStateAsync(current with
            {
                LastEventStreamConnectedAt = Connected ? now : current.LastEventStreamConnectedAt,
                LastEventGapAt = Connected ? current.LastEventGapAt : now
            }, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }
    }

    private static SwarmNodeRuntimeProjectionState CreateInitialRuntimeState(NodeKey key, DateTimeOffset now) => new(
        key.PlatformId,
        key.DockerNodeId,
        0,
        null,
        null,
        null,
        true,
        now,
        "Node inventory has not completed its first reconciliation.",
        null,
        null,
        null,
        null,
        null);
}

internal sealed class SwarmNodeLocalResourcesNotificationWorkItem(
    IDockerDaemonStreamManager streamManager,
    SwarmNodeLocalResourceSnapshot snapshot) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken) =>
        streamManager.SendSwarmNodeLocalResources(snapshot, cancellationToken);
}
