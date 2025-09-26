using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Runtime.CompilerServices;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Thread-safe bounded queue for scheduling asynchronous background tasks.
/// </summary>
internal interface IBackgroundTaskQueue
{
    void Enqueue(Func<CancellationToken, Task> workItem);
    ValueTask<Func<CancellationToken, Task>?> DequeueAsync(CancellationToken cancellationToken);
}

/// <inheritdoc/>
public sealed class BackgroundTaskQueue : IBackgroundTaskQueue
{
    private readonly Channel<Func<CancellationToken, Task>> queue 
        = Channel.CreateBounded<Func<CancellationToken, Task>>(Helpers.ChannelDefaultOptions());

    public void Enqueue(Func<CancellationToken, Task> workItem)
    {
        if (workItem == null) throw new ArgumentNullException(nameof(workItem));
        if (!queue.Writer.TryWrite(workItem))
            throw new InvalidOperationException("Queue is full");
    }

    public async ValueTask<Func<CancellationToken, Task>?> DequeueAsync(CancellationToken cancellationToken)
    {
        var workItem = await queue.Reader.ReadAsync(cancellationToken);
        return workItem;
    }
}

/// <summary>
/// A hosted background service that continuously dequeues and executes 
/// work items from an <see cref="IBackgroundTaskQueue"/> until the application 
/// shuts down or cancellation is requested.
/// 
/// This service decouples the scheduling of background work (from request handlers,
/// event processors, etc.) from its execution, ensuring that 
/// long-running or slow operations do not block request pipelines.
/// </summary>
internal sealed class QueuedHostedService(IBackgroundTaskQueue taskQueue, ILogger<QueuedHostedService> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        logger.LogInformation("Background task service running.");

        await foreach (var workItem in DequeueAllAsync(stoppingToken))
        {
            try
            {
                await workItem(stoppingToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred executing background task.");
            }
        }
    }

    private async IAsyncEnumerable<Func<CancellationToken, Task>> DequeueAllAsync([EnumeratorCancellation] CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            Func<CancellationToken, Task>? workItem;
            try
            {
                workItem = await taskQueue.DequeueAsync(stoppingToken);
            }
            catch (OperationCanceledException)
            {
                yield break;
            }

            if (workItem != null)
                yield return workItem;
        }
    }
}
