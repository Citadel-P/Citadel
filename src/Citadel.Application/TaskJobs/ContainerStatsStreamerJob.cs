using Application.Configs;
using Application.Mappers;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Grpc.Core;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.ObjectPool;
using Microsoft.Extensions.Options;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Collects containers stats from remote agents and pushes into the shared Channel <see cref="ContainerStatsWriterJob"/>.
/// </summary>  
internal class ContainerStatsStreamerJob(
    IOptions<JobConfiguration> options,
    ChannelWriter<ContainersStatBatch> channel,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerStatsStreamerJob> logger) : BackgroundService
{
    private readonly int _fetchIntervalMs = options.Value.MonitoringInterval * 1000;
    private readonly ConcurrentDictionary<string, StatsStreamRegistration> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();

    private readonly ObjectPool<List<ContainerStat>> _statsPool =
        new DefaultObjectPool<List<ContainerStat>>(new ContainerStatsListPooledObjectPolicy());

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var platform in _platformHealthReader.ReadAllAsync(cancellationToken))
            {
                if (platform.IsOnLine && platform.IsValidated)
                    StartStreamStatsForPlatform(platform, cancellationToken);
                else
                    await StopStreamStatsForPlatformAsync(platform.Address);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        finally
        {
            platformHealthBroadCaster.RemoveSubscriber(_platformHealthReader);
            var registrations = _runningStreams.Values.ToArray();
            foreach (var registration in registrations)
            {
                if (TryRemoveOwnedStream(registration.Address, registration))
                    registration.Cancel();
            }

            await Task.WhenAll(registrations.Select(static registration => registration.Task));
        }
    }

    private void StartStreamStatsForPlatform(PlatformHealth platform, CancellationToken cancellationToken)
    {
        var registration = new StatsStreamRegistration(
            platform.Address,
            CancellationTokenSource.CreateLinkedTokenSource(cancellationToken));

        if (!_runningStreams.TryAdd(platform.Address, registration))
        {
            registration.Dispose();
            logger.LogDebug("Streaming for {Address} is already active.", platform.Address);
            return;
        }

        registration.Task = StreamContainersStats(platform, registration);
    }

    private async Task StopStreamStatsForPlatformAsync(string address)
    {
        if (_runningStreams.TryRemove(address, out var registration))
        {
            logger.LogInformation("Stopping stream for {Address}", address);
            registration.Cancel();
            await registration.Task;
        }
    }

    private async Task StreamContainersStats(PlatformHealth platform, StatsStreamRegistration registration)
    {
        var cancellationToken = registration.Token;
        var command = new StreamContainersStatsCommand(platform.Address, _fetchIntervalMs);

        try
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    var connector = connectorFactory.GetConnector(platform.Type);
                    await foreach (var containers in connector.StreamContainersStatsAsync(command, cancellationToken))
                    {
                        if (containers.Count == 0) continue;
                        if (!platformContainerCache.TryGetContainers(platform.Id, out var ids)) continue;

                        var stats = _statsPool.Get();
                        var snapshotTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

                        foreach (var kvp in containers)
                        {
                            if (ids.TryGetValue(kvp.Key, out var containerId))
                                stats.Add(kvp.Value.Map(containerId, snapshotTime));
                        }

                        if (stats.Count == 0)
                        {
                            ReturnStatsList(stats);
                            continue;
                        }

                        var batch = new ContainersStatBatch(platform.Id, stats, ReturnStatsList);
                        var ownershipTransferred = false;
                        try
                        {
                            await channel.WriteAsync(batch, cancellationToken);
                            ownershipTransferred = true;
                        }
                        finally
                        {
                            if (!ownershipTransferred)
                                batch.Release();
                        }
                    }

                    if (!cancellationToken.IsCancellationRequested)
                    {
                        logger.LogWarning(
                            "Container stats stream for {Address} ended. Restarting in 10 seconds.",
                            platform.Address);
                    }
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    break;
                }
                catch (RpcException ex) when (
                    ex.StatusCode == StatusCode.Cancelled &&
                    cancellationToken.IsCancellationRequested)
                {
                    break;
                }
                catch (Exception ex)
                {
                    logger.LogError(
                        ex,
                        "Container stats stream failed for {Address}. Restarting in 10 seconds.",
                        platform.Address);
                }

                await DelayBeforeRestartAsync(cancellationToken);
            }
        }
        finally
        {
            TryRemoveOwnedStream(platform.Address, registration);
            registration.Dispose();
        }
    }

    private static async Task DelayBeforeRestartAsync(CancellationToken cancellationToken)
    {
        try
        {
            await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
    }

    private bool TryRemoveOwnedStream(string address, StatsStreamRegistration registration)
        => ((ICollection<KeyValuePair<string, StatsStreamRegistration>>)_runningStreams)
            .Remove(new KeyValuePair<string, StatsStreamRegistration>(address, registration));

    private void ReturnStatsList(List<ContainerStat> list)
    {
        _statsPool.Return(list);
    }

    private sealed class StatsStreamRegistration(
        string address,
        CancellationTokenSource cancellation) : IDisposable
    {
        public string Address { get; } = address;
        public CancellationToken Token => cancellation.Token;
        public Task Task { get; set; } = Task.CompletedTask;

        public void Cancel()
        {
            try { cancellation.Cancel(); } catch { }
        }

        public void Dispose() => cancellation.Dispose();
    }
}

public sealed class ContainersStatBatch(Guid platformId, List<ContainerStat> stats, Action<List<ContainerStat>> returnList)
{
    private Action<List<ContainerStat>>? _returnList = returnList;

    public Guid PlatformId { get; } = platformId;
    public List<ContainerStat> Stats { get; } = stats;
    public void Release() => Interlocked.Exchange(ref _returnList, null)?.Invoke(Stats);
}

internal sealed class ContainerStatsListPooledObjectPolicy : PooledObjectPolicy<List<ContainerStat>>
{
    internal const int MaxRetainedCapacity = 4096;

    public override List<ContainerStat> Create() => [];

    public override bool Return(List<ContainerStat> obj)
    {
        obj.Clear();
        return obj.Capacity <= MaxRetainedCapacity;
    }
}
