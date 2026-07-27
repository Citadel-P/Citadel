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
    private readonly ConcurrentDictionary<string, StatsStreamRegistration> _runningStreams = new();
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var platform in _platformHealthReader.ReadAllAsync(cancellationToken))
            {
                if (platform.IsOnLine)
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
            return;
        }

        registration.Task = StreamPlatformStats(
            platform.Id,
            platform.Type,
            platform.Address,
            registration);
    }

    private async Task StopStreamStatsForPlatformAsync(string address)
    {
        if (_runningStreams.TryRemove(address, out var registration))
        {
            logger.LogInformation("Stopping platform stream for {Address}", address);
            registration.Cancel();
            await registration.Task;
        }
    }

    private async Task StreamPlatformStats(
        Guid platformId,
        PlatformConnectorType connectorType,
        string address,
        StatsStreamRegistration registration)
    {
        var cancellationToken = registration.Token;
        var command = new StreamPlatformStatsCommand(address, _fetchIntervalMs);

        try
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    var connector = connectorFactory.GetConnector(connectorType);
                    await foreach (var platformStats in connector.StreamStatsAsync(command, cancellationToken))
                    {
                        await platformStatsWriter.WriteAsync((platformId, platformStats), cancellationToken);
                    }

                    if (!cancellationToken.IsCancellationRequested)
                    {
                        logger.LogWarning(
                            "Platform stats stream for {Address} ended. Restarting in 10 seconds.",
                            address);
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
                        "Platform stats stream failed for {Address}. Restarting in 10 seconds.",
                        address);
                }

                await DelayBeforeRestartAsync(cancellationToken);
            }
        }
        finally
        {
            TryRemoveOwnedStream(address, registration);
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
