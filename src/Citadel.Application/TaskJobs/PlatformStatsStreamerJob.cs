using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Grpc.Core;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Background service that manages the collection of real-time platform statistics from remote agents.
/// For each online platform, it starts a streaming task that gathers stats via the appropriate connector and writes <see cref="PlatformStatsResult"/>
/// data to a shared channel for persistence. When a platform goes offline, the corresponding stats stream is stopped.
/// </summary>
internal class PlatformStatsStreamerJob(
    IOptions<JobConfiguration> options,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ChannelWriter<(Guid Id, PooledHandle<PlatformStatsResult> Stats)> platformStatsWriter,
    ILogger<PlatformStatsStreamerJob> logger) : BackgroundService
{
    private readonly int _fetchIntervalMs = options.Value.SystemInfoInterval * 1000;
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
        _ = StreamPlatformStats(platform.Id, platform.Type, platform.Address, cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (_runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamPlatformStats(Guid platformId, PlatformConnectorType connectorType, string address, CancellationToken cancellationToken)
    {
        try
        {
            var connector = connectorFactory.GetConnector(connectorType);
            var command = new StreamPlatformStatsCommand(
                PlatformAddress: address,
                FetchIntervalMs: _fetchIntervalMs);

            await foreach (var platformStats in connector.StreamStatsAsync(command, cancellationToken))
            {
                await platformStatsWriter.WriteAsync((platformId, platformStats), cancellationToken);
            }
        }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
            logger.LogInformation("Stream for {Address} was canceled.", address);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while streaming container stats for {Address}", address);
            await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
        }
        finally
        {
            StopStreamStatsForPlatform(address);
        }
    }
}