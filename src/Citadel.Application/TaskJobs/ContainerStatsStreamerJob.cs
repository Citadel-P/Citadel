using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Configs;
using Application.Mappers;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Grpc.Core;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Collects containers stats from remote agents and pushes into the shared Channel <see cref="ContainerStatsWriterJob"/>.
/// </summary>
internal class ContainerStatsStreamerJob(
    IOptions<JobConfiguration> options,
    IObjectPoolManager objectPoolManager,
    ChannelWriter<ContainersStatBatch> channel,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerStatsStreamerJob> logger) : BackgroundService
{
    private readonly int _fetchIntervalMs = options.Value.ContainersInfoInterval * 1000;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platform in _platformHealthReader.ReadAllAsync(cancellationToken))
        {
            if (platform.IsOnLine)
            {
                StartStreamStatsForPlatform(platform, cancellationToken);
            }
            else
            {
                StopStreamStatsForPlatform(platform.Address);
            }
        }
    }

    public void StartStreamStatsForPlatform(PlatformHealth platform, CancellationToken cancellationToken)
    {
        
        if (!_runningStreams.TryAdd(platform.Address, CancellationTokenSource.CreateLinkedTokenSource(cancellationToken)))
        {
            logger.LogWarning("Streaming platform stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = _runningStreams[platform.Address];
        _ = StreamContainersStats(platform.Id, platform.Type, platform.Address, cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (_runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamContainersStats(Guid platformId, PlatformConnectorType connectorType, string address, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var command = new StreamContainersStatsCommand(
                    PlatformAddress: address, 
                    FetchIntervalMs: _fetchIntervalMs);

                await foreach (var pooledStats in connectorFactory.GetConnector(connectorType).StreamContainersStatsAsync(command, cancellationToken))
                {
                    using var _ = pooledStats;
                    var containers = pooledStats.Value;
                    try
                    {
                        if (containers.Count > 0)
                        {
                            if (platformContainerCache.TryGetContainers(platformId, out var ids))
                            {
                                var pooledStatsList = objectPoolManager.GetPooled<List<ContainerStat>>();
                                var stats = pooledStatsList.Value;
                                stats.Clear();

                                var snapshotTime = (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
                                foreach (var kvp in containers)
                                {
                                    if (ids.TryGetValue(kvp.Key, out var containerId))
                                    {
                                        var containerStat = objectPoolManager.Get<ContainerStat>();
                                        kvp.Value.Map(containerStat, containerId, snapshotTime);
                                        stats.Add(containerStat);
                                    }
                                }

                                await channel.WriteAsync(new ContainersStatBatch(platformId, pooledStatsList), cancellationToken);
                            }
                        }
                    }
                    finally
                    {
                        foreach (var s in pooledStats.Value.Values)
                        {
                            objectPoolManager.Return(s);
                        }
                    }
                }
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("Stream for {Address} was canceled.", address);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while streaming containers stats for {Address}, retrying in 10s...", address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
            finally
            {
                StopStreamStatsForPlatform(address);
            }
        }
    }
}

internal sealed record ContainersStatBatch(Guid PlatformId, PooledHandle<List<ContainerStat>> Stats);
