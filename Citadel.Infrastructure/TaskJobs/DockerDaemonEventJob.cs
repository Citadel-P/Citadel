using System.Collections.Concurrent;
using System.Threading.Channels;
using Grpc.Core;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

internal sealed class DockerDaemonEventJob(
    ILogger<DockerDaemonEventJob> logger,
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster) : BackgroundService
{
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platform in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            if (platform.IsOnLine)
            {
                StartMonitoringPlatform(platform, cancellationToken);
            }
            else
            {
                StopMonitoringPlatform(platform.Address);
            }
        }
    }

    public void StartMonitoringPlatform(PlatformHealth platform, CancellationToken cancellationToken)
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

    private async Task MonitorStream(PlatformHealth platform, CancellationToken cancellationToken)
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
                    if (reply.EventMessageType != Citadel.Agent.Containers.V1.EventMessageType.Container)
                        continue;


                    switch (reply.Action)
                    {
                        case "create":
                            var container = reply.Container.Map(platform.Id, DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                            dbContext.Containers.Add(container);
                            await dbContext.SaveChangesAsync(cancellationToken);
                            await containerHub.SendContainerEvent(container, reply.Action);
                            platformContainerCache.TryAddContainer(platform.Id, container.ContainerId, container.Id);
                            break;
                        case "destroy":
                            var existingDestroy = await dbContext.Containers.FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existingDestroy != null)
                            {
                                dbContext.Containers.Remove(existingDestroy);
                                await dbContext.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(existingDestroy, reply.Action);
                                platformContainerCache.TryRemoveContainer(platform.Id, existingDestroy.ContainerId);
                            }
                            break;
                        default:
                            var existing = await dbContext.Containers.FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
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

                                existing.PartialUpdate(state: ContainerMapper.Map(reply.Container.State));
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
