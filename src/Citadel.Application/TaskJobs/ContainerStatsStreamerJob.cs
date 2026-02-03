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
    private readonly ConcurrentDictionary<string, CancellationTokenSource> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();

    private readonly ObjectPool<List<ContainerStat>> _statsPool =
        new DefaultObjectPool<List<ContainerStat>>(new DefaultPooledObjectPolicy<List<ContainerStat>>());

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        // Listen for platform status changes
        await foreach (var platform in _platformHealthReader.ReadAllAsync(cancellationToken))
        {
            if (platform.IsOnLine)
                StartStreamStatsForPlatform(platform, cancellationToken);
            else
                StopStreamStatsForPlatform(platform.Address);
        }
    }

    private void StartStreamStatsForPlatform(PlatformHealth platform, CancellationToken cancellationToken)
    {
        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);

        if (!_runningStreams.TryAdd(platform.Address, cts))
        {
            cts.Dispose();
            logger.LogDebug("Streaming for {Address} is already active.", platform.Address);
            return;
        }

        // Fire and forget the stream task
        _ = StreamContainersStats(platform, cts.Token);
    }

    private void StopStreamStatsForPlatform(string address)
    {
        if (_runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Stopping stream for {Address}", address);
            cts.Cancel();
            cts.Dispose();
        }
    }

    private async Task StreamContainersStats(PlatformHealth platform, CancellationToken cancellationToken)
    {
        var command = new StreamContainersStatsCommand(platform.Address, _fetchIntervalMs);
        var connector = connectorFactory.GetConnector(platform.Type);

        try
        {
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

                if (stats.Count > 0)
                {
                    var batch = new ContainersStatBatch(platform.Id, stats, ReturnStatsList);
                    // Writer is responsible for calling batch.Release()
                    await channel.WriteAsync(batch, cancellationToken);
                }
                else
                {
                    ReturnStatsList(stats);
                }
            }
        }
        catch (OperationCanceledException) { /* Normal shutdown */ }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled) { /* Normal shutdown */ }
        catch (Exception ex)
        {
            logger.LogError(ex, "Stream error for {Address}. Restarting in 10s...", platform.Address);
            // We don't call StopStream here, we just cleanup and let it retry or exit so it can be restarted
            await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
        }
        finally
        {
            StopStreamStatsForPlatform(platform.Address);
        }
    }

    private void ReturnStatsList(List<ContainerStat> list)
    {
        list.Clear();
        _statsPool.Return(list);
    }
}

public sealed class ContainersStatBatch(Guid platformId, List<ContainerStat> stats, Action<List<ContainerStat>> returnList)
{
    public Guid PlatformId { get; } = platformId;
    public List<ContainerStat> Stats { get; } = stats;
    public void Release() => returnList(Stats);
}