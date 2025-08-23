using System.Collections.Concurrent;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Monitors external platforms (gRPC services) by periodically checking their availability and reporting their status.
/// </summary>
public interface IPlatformHealthMonitorJob : IHostedService
{
    bool TrackPlatform(string address, Guid id, PlatformConnectorType type);
    Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken);
}

internal class PlatformHealthMonitorJob(
    IServiceScopeFactory scopeFactory,
    IPlatformHealthBroadCaster broadcaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformHealthMonitorJob> logger) : BackgroundService, IPlatformHealthMonitorJob
{
    private readonly ConcurrentDictionary<string, PlatformTrackingInfo> trackedPlatforms = [];
    private readonly ConcurrentDictionary<string, bool> status = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);

    // Debounce settings
    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    private readonly ConcurrentDictionary<string, int> failureCounts = [];
    private readonly ConcurrentDictionary<string, int> successCounts = [];

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        // Create a scope and uow only for the initial fetch
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platforms = await uow.Platforms.GetPlatformsInfoAsync(cancellationToken);
            if (platforms.Any())
            {
                foreach (var platform in platforms)
                {
                    TrackPlatform(address: platform.Address, id: platform.Id, type: platform.ConnectorType);
                }
            }
        }

        while (!cancellationToken.IsCancellationRequested)
        {
            
            foreach (var (address, val) in trackedPlatforms)
            {
                var isOnline = (await connectorFactory.GetConnector(val.Type).CheckHealthAsync(address, cancellationToken)).Healthy;
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

    public bool TrackPlatform(string address, Guid id, PlatformConnectorType type)
        => trackedPlatforms.TryAdd(address, new PlatformTrackingInfo(Id: id, Type: type));

    public async Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken)
    {
        var removed = trackedPlatforms.TryRemove(address, out var platform) &&
           status.TryRemove(address, out _) &&
           successCounts.TryRemove(address, out _) &&
           failureCounts.TryRemove(address, out _);

        if (removed && platform is not null)
        {
            await broadcaster.PublishAsync(new PlatformHealth(platform.Id, address, platform.Type, false), cancellationToken);
        }

        return removed;
    }
         

    private async Task UpdateStatus(string address, bool isOnline, CancellationToken cancellationToken)
    {
        if (!trackedPlatforms.TryGetValue(address, out var platform)) return;

        status[address] = isOnline;
        await broadcaster.PublishAsync(new PlatformHealth(platform.Id, address, platform.Type, isOnline), cancellationToken);
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
}

public sealed record PlatformTrackingInfo(Guid Id, PlatformConnectorType Type);
public sealed record PlatformHealth(Guid Id, string Address, PlatformConnectorType Type, bool IsOnLine);