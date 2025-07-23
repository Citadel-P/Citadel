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
    IObjectPoolManager objectPoolManager,
    ChannelWriter<ContainersStatBatch> channel,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerStatsStreamerJob> logger) : BackgroundService
{
    private readonly JobConfiguration jobConfiguration = options.Value;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platform in platformHealthReader.ReadAllAsync(cancellationToken))
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
        
        if (!runningStreams.TryAdd(platform.Address, CancellationTokenSource.CreateLinkedTokenSource(cancellationToken)))
        {
            logger.LogWarning("Streaming platform stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = runningStreams[platform.Address];
        _ = StreamContainersStats(platform.Id, platform.Type, platform.Address, cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
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
                var command = new StreamContainerStatsCommand(
                    PlatformAddress: address, 
                    FetchIntervalMs: jobConfiguration.ContainersInfoInterval * 1000);

                await foreach (var stream in connectorFactory.GetConnector(connectorType).StreamContainerStatsAsync(command, cancellationToken))
                {
                    if (stream.Containers.Count > 0)
                    {
                        if (platformContainerCache.TryGetContainers(platformId, out var ids))
                        {
                            var stats = objectPoolManager.Get<List<ContainerStat>>();
                            var snapshotTime = (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
                            foreach (var kvp in stream.Containers)
                            {
                                if (ids.TryGetValue(kvp.Key, out var containerId))
                                {
                                    var containerStat = objectPoolManager.Get<ContainerStat>();
                                    kvp.Value.Map(containerStat, containerId, snapshotTime);
                                    stats.Add(containerStat);
                                }
                            }

                            await channel.WriteAsync(new ContainersStatBatch(platformId, stats), cancellationToken);
                         }

                        foreach (var s in stream.Containers.Values)
                            objectPoolManager.Return(s);

                        objectPoolManager.Return(stream);
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

internal sealed record ContainersStatBatch(Guid PlatformId, List<ContainerStat> Stats);
