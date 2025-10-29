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
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

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
    private readonly int _fetchIntervalMs = options.Value.ContainersInfoInterval * 1000;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();

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
            cts.Dispose();
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

                await foreach (var containers in connectorFactory.GetConnector(connectorType)
                    .StreamContainersStatsAsync(command, cancellationToken))
                {
                    if (containers.Count > 0 &&
                        platformContainerCache.TryGetContainers(platformId, out var ids))
                    {
                        var stats = new List<ContainerStat>(containers.Count);
                        var snapshotTime = (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;

                        foreach (var kvp in containers)
                        {
                            if (ids.TryGetValue(kvp.Key, out var containerId))
                            {
                                stats.Add(kvp.Value.Map(containerId, snapshotTime));
                            }
                        }

                        if (stats.Count > 0)
                        {
                            if (!channel.TryWrite(new ContainersStatBatch(platformId, stats)))
                            {
                                await channel.WriteAsync(new ContainersStatBatch(platformId, stats), cancellationToken);
                            }
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
                logger.LogError(ex, "Error while streaming container stats for {Address}, retrying in 10s...", address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
            finally
            {
                StopStreamStatsForPlatform(address);
            }
        }
    }
}

internal sealed record ContainersStatBatch(Guid PlatformId, List<ContainerStat> Stats);
