using System.Collections.Concurrent;
using System.Threading.Channels;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Grpc.Core;
using Infrastructure.Services;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Collects containers stats from remote agents and pushes into the shared Channel <see cref="ContainersStatsPersistenceJob"/>.
/// </summary>
internal class ContainersStatsCollectorJob(
    IOptions<JobConfiguration> options,
    ChannelWriter<ContainersStatBatch> channel,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainersStatsCollectorJob> logger) : BackgroundService
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
        if (runningStreams.ContainsKey(platform.Address))
        {
            logger.LogWarning("Streaming containers stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => StreamContainersStats(platform, cts.Token), cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamContainersStats(PlatformHealth platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var command = new StreamContainerStatsCommand(PlatformAddress: platform.Address, FetchIntervalMs: jobConfiguration.ContainersInfoInterval * 1000);
                await foreach (var stream in connectorFactory.GetConnector(platform.Type).StreamContainerStatsAsync(command, cancellationToken: cancellationToken))
                {
                    if (stream.Containers.Count > 0)
                    {
                        if (platformContainerCache.TryGetContainers(platform.Id, out var ids))
                        {
                            var snapshotTime = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                            foreach (var kvp in stream.Containers)
                            {
                                if (ids.TryGetValue(kvp.Key, out var containerId))
                                {
                                    kvp.Value.PartialUpdate(
                                        containerId: containerId,
                                        created: snapshotTime);
                                }
                            }

                            await channel.WriteAsync(new ContainersStatBatch(platform.Id, stream.Containers.Values), cancellationToken);
                        }
                    }
                }
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("Stream for {Address} was canceled.", platform.Address);
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while streaming containers stats for {Address}, retrying in 10s...", platform.Address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
        }
    }
}

internal sealed record ContainersStatBatch(Guid PlatformId, IEnumerable<ContainerStat> Stats);
