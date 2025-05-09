using System.Collections.Concurrent;
using Grpc.Core;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

public interface IDaemonEventJob
{
    void StartMonitoringPlatform(PlatformData platform, CancellationToken cancelationToken);
    void StopMonitoringPlatform(string address);
}

public sealed class DaemonEventJob(
    IServiceScopeFactory scopeFactory,
    IGrpcClientFactory clientFactory,
    ILogger<DaemonEventJob> logger) : BackgroundService, IDaemonEventJob
{
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platforms = await dbContext.Platforms.AsNoTracking()
            .Select(p => new PlatformData(p.Address, p.Id))
            .ToListAsync(stoppingToken);

        foreach (var platform in platforms)
        {
            StartMonitoringPlatform(platform, stoppingToken);
        }
    }

    public void StartMonitoringPlatform(PlatformData platform, CancellationToken cancelationToken)
    {
        if (runningStreams.ContainsKey(platform.Address))
        {
            logger.LogWarning("Stream for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancelationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => MonitorStream(platform, cts.Token), cts.Token);
    }

    public void StopMonitoringPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting stream for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task MonitorStream(PlatformData platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                using var scope = scopeFactory.CreateAsyncScope();
                var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

                var client = clientFactory.GetContainerClient(platform.Address);
                using var call = client.StreamDaemonEvent(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

                await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken))
                {
                    if (reply.EventMessageType != Agent.Server.Containers.EventMessageType.Container)
                        continue;

                    var containerInfo = reply.Container.Map(platform.PlatformId, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

                    switch (reply.Action)
                    {
                        case "create":
                            dbContext.ContainersInfo.Add(containerInfo);
                            break;
                        case "destroy":
                            var existingDestroy = await dbContext.ContainersInfo.FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existingDestroy != null) dbContext.ContainersInfo.Remove(existingDestroy);
                            break;
                        default:
                            var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existing != null)
                            {
                                var state = reply.Action switch
                                {
                                    "stop" => "exited",
                                    "start" => "running",
                                    "pause" => "paused",
                                    "restart" => "restarting",
                                    _ => throw new NotImplementedException()
                                };
                                existing.PartialUpdate(state: containerInfo.State, status: containerInfo.Status);
                            }
                            break;
                    }

                    await dbContext.SaveChangesAsync(cancellationToken);
                    await containerHub.SendContainerEvent(containerInfo, reply.Action);
                }
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("Stream for {Address} was cancelled.", platform.Address);
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while monitoring {Address}, retrying in 10s...", platform.Address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
        }
    }
}
public record PlatformData(string Address, Guid PlatformId);