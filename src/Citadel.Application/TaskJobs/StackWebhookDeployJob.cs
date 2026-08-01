using Application.Services;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal readonly record struct StackWebhookDeploySignal(Guid JobId);

internal sealed class StackWebhookDeployJob(
    ChannelReader<StackWebhookDeploySignal> reader,
    StackWebhookDeployQueueService queue,
    StackWebhookDeployProcessor processor,
    TimeProvider timeProvider,
    ILogger<StackWebhookDeployJob> logger) : BackgroundService
{
    private const int BatchSize = 20;
    private static readonly TimeSpan PollInterval = TimeSpan.FromSeconds(5);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        using var timer = new PeriodicTimer(PollInterval, timeProvider);
        var nextPoll = timer.WaitForNextTickAsync(stoppingToken).AsTask();
        var interruptedJobsRequeued = false;

        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                try
                {
                    if (!interruptedJobsRequeued)
                    {
                        await queue.RequeueInterruptedAsync(stoppingToken);
                        interruptedJobsRequeued = true;
                    }

                    await DrainReadyAsync(stoppingToken);
                }
                catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
                {
                    throw;
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Webhook stack deploy queue processing failed. Retrying on the next poll.");
                }

                var signal = reader.WaitToReadAsync(stoppingToken).AsTask();
                var completed = await Task.WhenAny(signal, nextPoll);
                if (completed == nextPoll)
                {
                    if (!await nextPoll)
                        break;

                    nextPoll = timer.WaitForNextTickAsync(stoppingToken).AsTask();
                    continue;
                }

                if (!await signal)
                    break;

                while (reader.TryRead(out _))
                {
                }
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
        {
        }
    }

    private async Task DrainReadyAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            var ids = await queue.GetReadyIdsAsync(BatchSize, cancellationToken);
            foreach (var id in ids)
                await processor.ProcessAsync(id, cancellationToken);

            if (ids.Count < BatchSize)
                return;
        }
    }
}
