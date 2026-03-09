using Application.Services;
using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;

namespace Application.TaskJobs;

public interface IPlatformHealthMonitorJob : IHostedService
{
    bool TrackPlatform(string address, Guid id, PlatformConnectorType type);
    Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken);
}

internal sealed class PlatformHealthMonitorJob(
    IAlertService alertService,
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
                        var platformName = await GetPlatformName(state.Id, ct);
                        logger.LogInformation(
                            "Platform {PlatformName} status changed to {Status}",
                            platformName,
                            isOnline ? "Online" : "Offline");

                        if (!isOnline)
                        {
                            await RaisePlatformUnreachableAlert(state.Id, platformName, address, ct);
                        }

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

    private async Task<string> GetPlatformName(Guid platformId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var info = await uow.Platforms.GetInfoAsync(platformId, ct);
        return info?.Name ?? string.Empty;
    }

    private async Task RaisePlatformUnreachableAlert(Guid platformId, string platformName, string address, CancellationToken ct)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms:
            [
                new PlatformAlertSnapshot(
                    Id: platformId,
                    Name: platformName,
                    CpuUsage: 0,
                    RamUsage: 0,
                    AgentVersion: string.Empty,
                    Address: address,
                    IsOnline: false)
            ],
            Deployments: [],
            Stacks: []);

        await alertService.ProcessAsync(
            AlertType.PlatformUnreachable,
            context,
            ct);
    }

    public bool TrackPlatform(string address, Guid id, PlatformConnectorType type)
        => platforms.TryAdd(address, new PlatformState(id, type));

    public async Task<bool> UntrackPlatform(string address, CancellationToken cancellationToken)
    {
        if (!platforms.TryRemove(address, out var state))
            return false;

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
