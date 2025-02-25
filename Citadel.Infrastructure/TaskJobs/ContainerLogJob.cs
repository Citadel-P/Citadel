using Agent.Server.Containers;
using Grpc.Core;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class ContainerLogJob(
     IGrpcClientFactory clientFactory,
     ApplicationDbContext dbContext,
     IContainerHubDispatcher containerHubDispatcher,
     ILogger<ContainerLogJob> logger) : IJob
{

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var containerId = context.MergedJobDataMap.GetString("ContainerId");
        var requestId = context.MergedJobDataMap.GetString("RequestId");
        if (string.IsNullOrEmpty(containerId) || string.IsNullOrEmpty(requestId))
        {
            throw new ArgumentNullException("ContainerId or RequestId is null or empty");
        }

        var platformAddress = await dbContext.ContainersInfo.GetPlatformAddress(containerId, context.CancellationToken);
        if (platformAddress == null)
        {
            logger.LogError("Platform doesn't exist for container {ContainerId}", containerId);
            return;
        }

        var client = clientFactory.GetContainerClient(platformAddress);
        var request = new ContainerLogRequest
        {
            ContainerId = containerId,
        };
        try
        {
            using var call = client.StreamContainerLogs(request, cancellationToken: context.CancellationToken);
            await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken: context.CancellationToken))
            {
                await containerHubDispatcher.SendContainerLogs(reply, requestId);
            }
        }
        catch (RpcException e)
        {
            if (e.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("StreamContainerLogs has been canceled");
            }
            else
            {
                logger.LogError(e, "Rpc exception {Message}", e.Message);
            }
        }
        catch (Exception e)
        {
            logger.LogError(e, "Fatal exception {Message}", e.Message);
        }
    }
}

public static class ContainerLogJobExtension
{
    public static JobKey GetJobKey(Guid requestId) => new(requestId.ToString(), "ContainerLog");

    public static async Task EnqueueContainerLogsJob(this IScheduler scheduler, string containerId, Guid requestId, ILogger logger, CancellationToken cancellationToken)
    {
        var jobKey = GetJobKey(requestId);
        try
        {
            if (!await scheduler.CheckExists(jobKey, cancellationToken))
            {
                var job = JobBuilder.Create<ContainerLogJob>()
                                            .WithIdentity(jobKey)
                                            .UsingJobData("ContainerId", containerId)
                                            .UsingJobData("RequestId", requestId.ToString())
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

    public static async Task AbortContainerLogsJob(this IScheduler scheduler, Guid requestId, ILogger logger, CancellationToken cancellationToken)
    {
        var jobKey = GetJobKey(requestId);
        if (await scheduler.CheckExists(jobKey, cancellationToken))
        {
            await scheduler.Interrupt(GetJobKey(requestId), cancellationToken);
            logger.LogInformation("Job {JobKey} has been aborted", jobKey);
        }
    }
}