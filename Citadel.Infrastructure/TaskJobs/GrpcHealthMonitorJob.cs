using System.Collections.Concurrent;
using System.Threading.Channels;
using Infrastructure.Services.Abstractions;
using Microsoft.Extensions.Hosting;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Monitor gRPC services (Docker Agents) health status.
/// </summary>
public interface IGrpcHealthMonitorJob : IHostedService
{
    bool IsServiceOnLine(string address);
    void TrackAddress(string address);
    void TrackAddress(string[] addresses);
    void UntrackAddress(string address);
}

internal class GrpcHealthMonitorJob(
    IGrpcClientFactory clientFactory, 
    ChannelWriter<GrpcServiceHealth> channelWriter) : BackgroundService, IGrpcHealthMonitorJob
{
    private readonly Lock @lock = new();
    private readonly List<string> trackedAddresses = [];
    private readonly ConcurrentDictionary<string, bool> status = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);

    // debounce settings
    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    private readonly Dictionary<string, int> failureCounts = [];
    private readonly Dictionary<string, int> successCounts = [];

    
    public void TrackAddress(string address)
    {
        lock (@lock)
        {
            if (!trackedAddresses.Contains(address))
                trackedAddresses.Add(address);
        }
    }

    public void TrackAddress(string[] addresses)
    {
        lock (@lock)
        {
            foreach (var address in addresses)
            {
                if (!trackedAddresses.Contains(address))
                    trackedAddresses.AddRange(address);
            }
        }
    }

    public void UntrackAddress(string address)
    {
        lock (@lock)
        {
            if (trackedAddresses.Any(s => s == address))
            {
                trackedAddresses.Remove(address);
                status.TryRemove(address, out _);
            }
        }
    }

    public bool IsServiceOnLine(string address) => status.TryGetValue(address, out var online) && online;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            string[] addresses;
            lock (@lock)
            {
                addresses = [.. trackedAddresses];
            }

            foreach (var address in addresses)
            {
                var isOnline = await ProbeAsync(address);
                var wasOnline = status.TryGetValue(address, out var prevOnline) && prevOnline;

                if (isOnline)
                {
                    failureCounts[address] = 0;

                    successCounts[address] = successCounts.GetValueOrDefault(address) + 1;
                    if (!wasOnline && successCounts[address] >= SuccessThreshold)
                    {
                        UpdateStatus(address, isOnline, cancellationToken);
                    }
                }
                else
                {
                    successCounts[address] = 0;

                    failureCounts[address] = failureCounts.GetValueOrDefault(address) + 1;
                    if (wasOnline && failureCounts[address] >= FailThreshold)
                    {
                        UpdateStatus(address, isOnline, cancellationToken);
                    }
                }
            }

            await Task.Delay(checkInterval, cancellationToken);
        }

        channelWriter.TryComplete();
    }

    private async void UpdateStatus(string address, bool isOnline, CancellationToken cancellationToken)
    {
        status[address] = isOnline;
        var evt = new GrpcServiceHealth(address, isOnline);

        await channelWriter.WriteAsync(evt, cancellationToken);
    }

    private async Task<bool> ProbeAsync(string address)
    {
        try
        {
            var client = clientFactory.GetPlatformClient(address);
            var response = await client.HealthCheckAsync(new Google.Protobuf.WellKnownTypes.Empty(), deadline: DateTime.UtcNow.AddSeconds(2));
            return response.Healthy;
        }
        catch
        {
            return false;
        }
    }
}

internal sealed record GrpcServiceHealth(string Address, bool IsOnLine);