using System.Collections.Concurrent;
using System.Threading.Channels;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Monitors the health of gRPC services (Platforms) by periodically checking their availability and reporting their status.
/// </summary>
public interface IPlatformHealthMonitorJob : IHostedService
{
    bool IsPlatformOnLine(string address);
    void TrackPlatform(string address);
    void TrackPlatforms(string[] addresses);
    void UntrackPlatform(string address);
}

internal class PlatformHealthMonitorJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    ChannelWriter<GrpcServiceHealth> channelWriter) : BackgroundService, IPlatformHealthMonitorJob
{
    private readonly Lock @lock = new();
    private readonly List<string> trackedAddresses = [];
    private readonly ConcurrentDictionary<string, bool> status = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);

    // Debounce settings
    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    private readonly Dictionary<string, int> failureCounts = [];
    private readonly Dictionary<string, int> successCounts = [];

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        // Create a scope and dbContext only for the initial fetch
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            var addresses = await dbContext.Platforms.AsNoTracking().Select(s => s.Address).ToArrayAsync(cancellationToken);
            if (addresses != null && addresses.Length > 0)
            {
                TrackPlatforms(addresses);
            }
        }

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
                var hasPreviousStatus = status.TryGetValue(address, out var wasOnline);

                // If we've never seen this address before, initialize and emit
                if (!hasPreviousStatus)
                {
                    UpdateStatus(address, isOnline, cancellationToken);
                    continue;
                }

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

    public void TrackPlatform(string address)
    {
        lock (@lock)
        {
            if (!trackedAddresses.Contains(address))
                trackedAddresses.Add(address);
        }
    }

    public void TrackPlatforms(string[] addresses)
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

    public void UntrackPlatform(string address)
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

    public bool IsPlatformOnLine(string address) => status.TryGetValue(address, out var online) && online;

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