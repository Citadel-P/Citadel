using System.Collections.Concurrent;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

public interface IPlatformHealthMonitorJob : IHostedService
{
    bool TrackPlatform(string address, Guid id, PlatformConnectorType type);
    Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken);
}

internal sealed class PlatformHealthMonitorJob(
    IServiceScopeFactory scopeFactory,
    IPlatformHealthBroadCaster broadcaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformHealthMonitorJob> logger)
    : BackgroundService, IPlatformHealthMonitorJob
{
    private readonly ConcurrentDictionary<string, PlatformState> platforms = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);
    private readonly TimeSpan healthTimeout = TimeSpan.FromSeconds(2);

    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await LoadFromDatabase(stoppingToken);

        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = stoppingToken
        };

        while (!stoppingToken.IsCancellationRequested)
        {
            await Parallel.ForEachAsync(platforms, parallelOptions, async (entry, ct) =>
            {
                var address = entry.Key;
                var state = entry.Value;

                bool isOnline;
                try
                {
                    using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
                    timeoutCts.CancelAfter(healthTimeout);

                    var connector = connectorFactory.GetConnector(state.Type);
                    var result = await connector.CheckHealthAsync(address, timeoutCts.Token);
                    isOnline = result.Healthy;
                }
                catch (OperationCanceledException)
                {
                    return; // shutdown or timeout -> skip this cycle
                }
                catch (Exception ex)
                {
                    logger.LogWarning(ex, "Health check failed for {Address}", address);
                    isOnline = false;
                }

                if (state.TryUpdate(isOnline, FailThreshold, SuccessThreshold, out var shouldEmit))
                {
                    if (shouldEmit)
                    {
                        logger.LogInformation(
                            "Platform {Address} status changed to {Status}",
                            address,
                            isOnline ? "Online" : "Offline");

                        await broadcaster.PublishAsync(
                            new PlatformHealth(state.Id, address, state.Type, isOnline),
                            ct);
                    }
                }
            });

            await Task.Delay(checkInterval, stoppingToken);
        }

        broadcaster.Complete();
    }

    private async Task LoadFromDatabase(CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platformsInfo = await uow.Platforms.GetPlatformsInfoAsync(ct);

        foreach (var p in platformsInfo)
        {
            TrackPlatform(p.Address, p.Id, p.ConnectorType);
        }
    }

    public bool TrackPlatform(string address, Guid id, PlatformConnectorType type)
        => platforms.TryAdd(address, new PlatformState(id, type));

    public async Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken)
    {
        if (!platforms.TryRemove(address, out var state))
            return false;

        // Todo: tell UI it disappeared instead of "offline"
        await broadcaster.PublishAsync(
            new PlatformHealth(state.Id, address, state.Type, false), cancellationToken);

        return true;
    }
}

internal sealed class PlatformState(Guid id, PlatformConnectorType type)
{
    public Guid Id { get; } = id;
    public PlatformConnectorType Type { get; } = type;

    private bool? lastStatus;
    private int successCount;
    private int failureCount;
    private readonly Lock @lock = new();

    public bool TryUpdate(
        bool isOnline,
        int failThreshold,
        int successThreshold,
        out bool shouldEmit)
    {
        using (@lock.EnterScope())
        {
            shouldEmit = false;

            if (isOnline)
            {
                failureCount = 0;
                successCount++;

                if (lastStatus != true && successCount >= successThreshold)
                {
                    lastStatus = true;
                    shouldEmit = true;
                }
            }
            else
            {
                successCount = 0;
                failureCount++;

                if (lastStatus != false && failureCount >= failThreshold)
                {
                    lastStatus = false;
                    shouldEmit = true;
                }
            }

            return true;
        }
    }
}

public sealed record PlatformHealth(Guid Id, string Address, PlatformConnectorType Type, bool IsOnLine);
