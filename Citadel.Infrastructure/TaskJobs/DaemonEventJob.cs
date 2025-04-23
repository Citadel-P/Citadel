using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class DaemonEventJob(
    ApplicationDbContext dbContext,
    ISchedulerFactory schedulerFactory,
    ILogger<DaemonEventJob> logger) : IJob
{
    public static readonly JobKey JobKey = new(nameof(DaemonEventJob), "DaemonEvent");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var scheduler = await schedulerFactory.GetScheduler(context.CancellationToken);
        var platforms = await dbContext.Platforms.AsNoTracking().Select(s => new PlatformData(s.Address, s.Id)).ToListAsync(context.CancellationToken);
        if (platforms.Count == 0) return;

        foreach (var platform in platforms)
        {
            await scheduler.EnqueueStreamDaemonEventJob(platform, logger, context.CancellationToken);
        }
    }
}

internal class StreamDaemonEventJob(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext, ILogger<StreamDaemonEventJob> logger,
    IContainerHubDispatcher containerHub) : IJob
{
    private static readonly HashSet<string> ContainerEventNames = ["create", "destroy", "stop", "start", "pause", "restart"];

    public async ValueTask Execute(IJobExecutionContext context)
    {    
        try
        {
            if (!Guid.TryParse(context.MergedJobDataMap.GetString("platformId"), out var platformId) || platformId == Guid.Empty)
            {
                logger.LogError("No PlatformId has been set in job params");
                return;
            }
            var address = context.MergedJobDataMap.GetString("address");
            var client = clientFactory.GetContainerClient(address);

            using var call = client.StreamDaemonEvent(new Empty(), cancellationToken: context.CancellationToken);
            await foreach (var reply in call.ResponseStream.ReadAllAsync(context.CancellationToken))
            {

                if (reply.EventMessageType == Agent.Server.Containers.EventMessageType.Container)
                {
                    var containerInfo = reply.Container.Map(platformId, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

                    if (reply.Action == "create")
                    {
                        dbContext.ContainersInfo.Add(containerInfo);
                    }
                    else if (reply.Action == "destroy")
                    {
                        var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == reply.ContainerId, context.CancellationToken);
                        dbContext.ContainersInfo.Remove(existing);
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
                            existing.PartialUpdate(state: containerInfo.State, status: containerInfo.Status);
                        }
                    }

                    await dbContext.SaveChangesAsync(context.CancellationToken);
                    await containerHub.SendContainerEvent(containerInfo, reply.Action);
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

    public static async Task EnqueueStreamDaemonEventJob(this IScheduler scheduler, PlatformData platformData, ILogger logger, CancellationToken cancellationToken)
    {
        var jobKey = GetJobKey(platformData.Address);
        try
        {
            if (!await scheduler.CheckExists(jobKey, cancellationToken))
            {
                var job = JobBuilder.Create<StreamDaemonEventJob>()
                                            .WithIdentity(jobKey)
                                            .UsingJobData("address", platformData.Address)
                                            .UsingJobData("platformId", platformData.PlatformId.ToString())
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

public record PlatformData(string Address, Guid PlatformId);