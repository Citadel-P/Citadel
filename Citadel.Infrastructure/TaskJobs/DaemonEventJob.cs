using System.Collections.Concurrent;
using Grpc.Core;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

public interface IDaemonEventJob : IHostedService
{
    void StartMonitoringPlatform(PlatformData platform, CancellationToken cancelationToken);
    void StopMonitoringPlatform(string address);
}

internal sealed class DaemonEventJob(
    IServiceScopeFactory scopeFactory,
    IGrpcClientFactory clientFactory,
    ILogger<DaemonEventJob> logger) : BackgroundService, IDaemonEventJob
{
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platforms = await dbContext.Platforms.AsNoTracking()
                 .Select(s => new PlatformData(s.Id, s.Address, s.Status))
                 .ToListAsync(cancellationToken);

        foreach (var platform in platforms)
        {
            StartMonitoringPlatform(platform, cancellationToken);
        }
    }

    public void StartMonitoringPlatform(PlatformData platform, CancellationToken cancellationToken)
    {
        if (runningStreams.ContainsKey(platform.Address))
        {
            logger.LogWarning("Monitoring daemon events for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => MonitorStream(platform, cts.Token), cts.Token);
    }

    public void StopMonitoringPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting monitoring daemon events for {Address}", address);
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
                using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

                var client = clientFactory.GetContainerClient(platform.Address);
                using var call = client.StreamDaemonEvent(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

                await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken))
                {
                    if (reply.EventMessageType != Agent.Server.Containers.EventMessageType.Container)
                        continue;


                    switch (reply.Action)
                    {
                        case "create":
                            var containerInfo = reply.Container.Map(platform.Id, DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                            dbContext.ContainersInfo.Add(containerInfo);
                            await dbContext.SaveChangesAsync(cancellationToken);
                            await containerHub.SendContainerEvent(containerInfo, reply.Action);
                            break;
                        case "destroy":
                            var existingDestroy = await dbContext.ContainersInfo.FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existingDestroy != null)
                            {
                                dbContext.ContainersInfo.Remove(existingDestroy);
                                await dbContext.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(existingDestroy, reply.Action);
                            }
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

                                existing.PartialUpdate(state: ContainerInfoMapper.Map(reply.Container.State));
                                await dbContext.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(existing, reply.Action);
                            }
                            break;
                    }
                }
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("Stream for {Address} was canceled.", platform.Address);
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
