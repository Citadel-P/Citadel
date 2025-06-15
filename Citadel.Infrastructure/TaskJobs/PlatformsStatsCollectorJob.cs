using System.Collections.Concurrent;
using System.Threading.Channels;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Grpc.Core;
using Infrastructure.Services;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Collects platforms stats from remote agents and pushes into the shared Channel <see cref="PlatformsStatsPersistenceJob"/>.
/// </summary>
internal class PlatformsStatsCollectorJob(
    IOptions<JobConfiguration> options,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ChannelWriter<PlatformStatsBatch> platformStatsWriter,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformsStatsCollectorJob> logger) : BackgroundService
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
            logger.LogWarning("Streaming platform stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => StreamPlatformStats(platform, cts.Token), cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamPlatformStats(PlatformHealth platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var command = new StreamPlatformStatsCommand(
                    PlatformId: platform.Id,
                    PlatformAddress: platform.Address,
                    FetchIntervalMs: jobConfiguration.SystemInfoInterval * 1000);
                
                await foreach (var batch in connectorFactory.GetConnector(platform.Type).StreamStatsAsync(command, cancellationToken))
                {
                    await platformStatsWriter.WriteAsync(batch, cancellationToken);
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