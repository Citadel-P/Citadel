using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.Services;
using Microsoft.Extensions.Logging;
using Quartz;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Microsoft.EntityFrameworkCore;

namespace Infrastructure.TaskJobs;

internal class DaemonEventJob(
    ICacheService cacheService,
    ISchedulerFactory schedulerFactory,
    ILogger<DaemonEventJob> logger) : IJob
{
    public static readonly JobKey JobKey = new(nameof(DaemonEventJob), "DaemonEvent");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var scheduler = await schedulerFactory.GetScheduler(context.CancellationToken);
        var addresses = await cacheService.GetClientsAddresses(context.CancellationToken);
        if (!addresses.Any()) return;

        foreach (var address in addresses)
        {
            await scheduler.EnqueueStreamDaemonEventJob(address, logger, context.CancellationToken);
        }
    }
}

internal class StreamDaemonEventJob(
    ICacheService cacheService,
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<StreamDaemonEventJob> logger,
    IContainerHubDispatcher containerHub) : IJob
{
    public async ValueTask Execute(IJobExecutionContext context)
    {    
        try
        {
            var address = context.MergedJobDataMap.GetString("address");
            var client = clientFactory.GetContainerClient(address);

            using var call = client.StreamDaemonEvent(new Empty(), cancellationToken: context.CancellationToken);
            await foreach (var reply in call.ResponseStream.ReadAllAsync(context.CancellationToken).ConfigureAwait(false))
            {
                Guid? platformId = await cacheService.GetPlatformId(reply.Id, context.CancellationToken);
                if (platformId == null)
                {
                    logger.LogError("Platform does not exists, {daemonId}:", reply.Id);
                }

                if (reply.EventMessageType == Agent.Server.Containers.EventMessageType.Container)
                {
                    var containerInfo = reply.Container.Map(platformId.Value, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

                    if (reply.Action == "create")
                    {
                        dbContext.ContainersInfo.Add(containerInfo);
                        await dbContext.SaveChangesAsync(context.CancellationToken);
                    }
                    else if (reply.Action == "destroy")
                    {
                        var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == reply.ContainerId, context.CancellationToken);
                        dbContext.ContainersInfo.Remove(existing);
                        await dbContext.SaveChangesAsync(context.CancellationToken);
                    }
                    else
                    {
                        var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == reply.ContainerId, context.CancellationToken);
                        if (existing != null)
                        {
                            var state = (reply.Action) switch
                            {
                                "stop" => "exited",
                                "start" => "running",
                                "pause" => "paused",
                                "restart" => "restarting",
                                _ => throw new NotImplementedException()
                            };
                            existing.PartialUpdate(state: state, status: containerInfo.Status);

                            await dbContext.SaveChangesAsync(context.CancellationToken);
                        }
                    }

                    var containers = await dbContext.ContainersInfo.WithLastStat(platformId.Value).ToListAsync(context.CancellationToken);
                    await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));
                }
            }
        }
        catch (Exception ex)
        {
            if (ex is RpcException rpc && rpc.Status.StatusCode == StatusCode.Cancelled) {
                // In case the job was canceled
                logger.LogInformation("Job was canceled");
                return;
            }
            logger.LogError(ex, "An exception occurred while streaming daemon events");
            await RescheduleJob(context);
        }
    }

    private static async Task RescheduleJob(IJobExecutionContext context)
    {
        var oldTrigger = context.Trigger;
        var newTrigger = TriggerBuilder.Create()
            .ForJob(context.JobDetail)
            .WithIdentity($"{oldTrigger.Key.Name}-retry", oldTrigger.Key.Group)
            .StartAt(DateTimeOffset.UtcNow.AddSeconds(10))
            .Build();
        await context.Scheduler.ScheduleJob(newTrigger);
    }
}

public static class StreamDaemonEventJobExtension
{
    public static JobKey GetJobKey(string address) => new(address, "StreamDaemonEvent");

    public static async Task EnqueueStreamDaemonEventJob(this IScheduler scheduler, string address, ILogger logger, CancellationToken cancellationToken)
    {
        var jobKey = GetJobKey(address);
        try
        {
            if (!await scheduler.CheckExists(jobKey, cancellationToken))
            {
                var job = JobBuilder.Create<StreamDaemonEventJob>()
                                            .WithIdentity(jobKey)
                                            .UsingJobData("address", address)
                                            .Build();

                var trigger = TriggerBuilder.Create()
                                .ForJob(job)
                                .WithSimpleSchedule()
                                .StartNow()
                                .Build();

                await scheduler.ScheduleJob(job, trigger, cancellationToken);
                logger.LogInformation("Job {JobKey} has been scheduled", jobKey);

            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "{Message}", ex.Message);
        }
    }

    public static async Task AbortStreamDaemonEventJob(this IScheduler scheduler, string address, ILogger logger, CancellationToken cancellationToken)
    {
        var jobKey = GetJobKey(address);
        if (await scheduler.CheckExists(jobKey, cancellationToken))
        {
            await scheduler.Interrupt(GetJobKey(address), cancellationToken);
            logger.LogInformation("Job {JobKey} has been aborted", jobKey);
        }
    }
}