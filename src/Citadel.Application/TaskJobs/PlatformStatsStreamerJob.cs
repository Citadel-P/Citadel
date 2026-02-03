using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Grpc.Core;
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
    ChannelWriter<(Guid Id, PlatformStatsResult Stats)> platformStatsWriter,
    ILogger<PlatformStatsStreamerJob> logger) : BackgroundService
{
    private readonly int _fetchIntervalMs = options.Value.MonitoringInterval * 1000;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
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
            cts.Dispose(); // Prevent leak
            return;
        }

        _ = StreamPlatformStats(platform.Id, platform.Type, platform.Address, cts.Token);
    }

    private void StopStreamStatsForPlatform(string address)
    {
        if (_runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Stopping platform stream for {Address}", address);
            cts.Cancel();
            cts.Dispose();
        }
    }

    private async Task StreamPlatformStats(Guid platformId, PlatformConnectorType connectorType, string address, CancellationToken cancellationToken)
    {
        try
        {
            var connector = connectorFactory.GetConnector(connectorType);
            var command = new StreamPlatformStatsCommand(address, _fetchIntervalMs);

            await foreach (var platformStats in connector.StreamStatsAsync(command, cancellationToken))
            {
                await platformStatsWriter.WriteAsync((platformId, platformStats), cancellationToken);
            }
        }
        catch (OperationCanceledException) { }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while streaming platform stats for {Address}. Retrying in 10s...", address);
            await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
        }
        finally
        {
            StopStreamStatsForPlatform(address);
        }
    }
}