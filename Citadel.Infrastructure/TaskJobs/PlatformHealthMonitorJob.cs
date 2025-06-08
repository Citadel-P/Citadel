using System.Collections.Concurrent;
using System.Threading;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Monitors the external Platforms (gRPC services) by periodically checking their availability and reporting their status.
/// </summary>
public interface IPlatformHealthMonitorJob : IHostedService
{
    bool TrackPlatform(string address, Guid id);
    Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken);
}

internal class PlatformHealthMonitorJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IPlatformHealthBroadCaster broadcaster,
    ILogger<PlatformHealthMonitorJob> logger) : BackgroundService, IPlatformHealthMonitorJob
{
    private readonly ConcurrentDictionary<string, Guid> trackedAddresses = [];
    private readonly ConcurrentDictionary<string, bool> status = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);

    // Debounce settings
    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    private readonly ConcurrentDictionary<string, int> failureCounts = [];
    private readonly ConcurrentDictionary<string, int> successCounts = [];

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        // Create a scope and dbContext only for the initial fetch
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            var platforms = await dbContext.Platforms
                .AsNoTracking()
                .Select(s => new { s.Id, s.Address })
                .ToArrayAsync(cancellationToken);

            if (platforms != null && platforms.Length > 0)
            {
                foreach (var platform in platforms)
                {
                    TrackPlatform(platform.Address, platform.Id);
                }
            }
        }

        while (!cancellationToken.IsCancellationRequested)
        {
            
            foreach (var address in trackedAddresses.Keys)
            {
                var isOnline = await ProbeAsync(address, cancellationToken);
                var hasPreviousStatus = status.TryGetValue(address, out var wasOnline);

                // If we've never seen this address before, initialize and emit
                if (!hasPreviousStatus)
                {
                    await UpdateStatus(address, isOnline, cancellationToken);
                    continue;
                }

                if (ShouldEmitUpdate(address, isOnline, wasOnline))
                {
                    logger.LogInformation("Platform {Address} status changed to {Status}", address, isOnline ? "Online" : "Offline");
                    await UpdateStatus(address, isOnline, cancellationToken);
                }
            }

            await Task.Delay(checkInterval, cancellationToken);
        }

        broadcaster.Complete();
    }

    public bool TrackPlatform(string address, Guid id) => trackedAddresses.TryAdd(address, id);

    public async Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken)
    {
        var removed = trackedAddresses.TryRemove(address, out var id) &&
           status.TryRemove(address, out _) &&
           successCounts.TryRemove(address, out _) &&
           failureCounts.TryRemove(address, out _);

        if (removed)
        {
            await broadcaster.BroadcastAsync(new PlatformHealth(id, address, false), cancellationToken);
        }

        return removed;
    }
         

    private async Task UpdateStatus(string address, bool isOnline, CancellationToken cancellationToken)
    {
        if (!trackedAddresses.TryGetValue(address, out var platformId)) return;

        status[address] = isOnline;
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, address, isOnline), cancellationToken);
    }

    private bool ShouldEmitUpdate(string address, bool isOnline, bool wasOnline)
    {
        if (isOnline)
        {
            failureCounts[address] = 0;
            successCounts[address] = successCounts.GetOrAdd(address, 0) + 1;
            return !wasOnline && successCounts[address] >= SuccessThreshold;
        }
        else
        {
            successCounts[address] = 0;
            failureCounts[address] = failureCounts.GetOrAdd(address, 0) + 1;
            return wasOnline && failureCounts[address] >= FailThreshold;
        }
    }

    private async Task<bool> ProbeAsync(string address, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetPlatformClient(address);
            var response = await client.HealthCheckAsync(new Google.Protobuf.WellKnownTypes.Empty(), deadline: DateTime.UtcNow.AddSeconds(2), cancellationToken: cancellationToken);
            return response.Healthy;
        }
        catch
        {
            return false;
        }
    }
}

public sealed record PlatformHealth(Guid Id, string Address, bool IsOnLine);