using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class UnmanagedContainerAlertJob(
    ChannelReader<UnmanagedContainerAlertRequest> reader,
    IServiceScopeFactory scopeFactory,
    IAlertService alertService,
    TimeProvider timeProvider,
    ILogger<UnmanagedContainerAlertJob> logger) : BackgroundService
{
    private static readonly TimeSpan GracePeriod = TimeSpan.FromSeconds(30);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var processingChannel = Channel.CreateBounded<UnmanagedContainerAlertRequest>(
            new BoundedChannelOptions(256)
            {
                SingleReader = true,
                SingleWriter = true,
                AllowSynchronousContinuations = false,
                FullMode = BoundedChannelFullMode.Wait
            });
        var processorTask = ProcessAlertsAsync(processingChannel.Reader, stoppingToken);

        try
        {
            await ScheduleAlertsAsync(processingChannel.Writer, stoppingToken);
        }
        finally
        {
            processingChannel.Writer.TryComplete();
            try { await processorTask; }
            catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        }
    }

    private async Task ScheduleAlertsAsync(
        ChannelWriter<UnmanagedContainerAlertRequest> processingWriter,
        CancellationToken stoppingToken)
    {
        var pending = new Dictionary<UnmanagedContainerAlertKey, ScheduledUnmanagedContainerAlert>();
        var schedule = new PriorityQueue<ScheduledUnmanagedContainerAlert, long>();
        var readerCompleted = false;

        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                while (reader.TryRead(out var request))
                {
                    var scheduled = new ScheduledUnmanagedContainerAlert(
                        request,
                        timeProvider.GetUtcNow().Add(GracePeriod));
                    pending[scheduled.Key] = scheduled;
                    schedule.Enqueue(scheduled, scheduled.DueAt.UtcTicks);
                }

                await DispatchDueAlertsAsync(schedule, pending, processingWriter, stoppingToken);

                if (readerCompleted && schedule.Count == 0)
                    break;

                if (schedule.TryPeek(out _, out var dueTicks))
                {
                    var delay = TimeSpan.FromTicks(Math.Max(0, dueTicks - timeProvider.GetUtcNow().UtcTicks));
                    using var waitCancellation = CancellationTokenSource.CreateLinkedTokenSource(stoppingToken);
                    var readTask = readerCompleted
                        ? null
                        : reader.WaitToReadAsync(waitCancellation.Token).AsTask();
                    var delayTask = Task.Delay(delay, timeProvider, waitCancellation.Token);
                    var completed = readTask is null
                        ? await Task.WhenAny(delayTask)
                        : await Task.WhenAny(readTask, delayTask);

                    if (readTask is not null
                        && completed == readTask
                        && readTask.IsCompletedSuccessfully
                        && !readTask.Result)
                    {
                        readerCompleted = true;
                    }

                    waitCancellation.Cancel();
                    if (readTask is not null)
                        await ObserveCancellationAsync(readTask);
                    await ObserveCancellationAsync(delayTask);
                }
                else
                {
                    if (!await reader.WaitToReadAsync(stoppingToken))
                        break;
                }
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
    }

    private async Task DispatchDueAlertsAsync(
        PriorityQueue<ScheduledUnmanagedContainerAlert, long> schedule,
        Dictionary<UnmanagedContainerAlertKey, ScheduledUnmanagedContainerAlert> pending,
        ChannelWriter<UnmanagedContainerAlertRequest> processingWriter,
        CancellationToken cancellationToken)
    {
        var nowTicks = timeProvider.GetUtcNow().UtcTicks;
        while (schedule.TryPeek(out var scheduled, out var dueTicks) && dueTicks <= nowTicks)
        {
            schedule.Dequeue();
            if (!pending.TryGetValue(scheduled.Key, out var latest) || latest.DueAt != scheduled.DueAt)
                continue;

            pending.Remove(scheduled.Key);
            await processingWriter.WriteAsync(scheduled.Request, cancellationToken);
        }
    }

    private async Task ProcessAlertsAsync(
        ChannelReader<UnmanagedContainerAlertRequest> processingReader,
        CancellationToken cancellationToken)
    {
        await foreach (var request in processingReader.ReadAllAsync(cancellationToken))
        {
            try
            {
                await ProcessAsync(request, cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Failed to process unmanaged container alert for {DockerContainerId} on platform {PlatformId}",
                    request.DockerContainerId,
                    request.PlatformId);
            }
        }
    }

    private async Task ProcessAsync(UnmanagedContainerAlertRequest request, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var container = await uow.Containers.GetByIdAsync(request.DockerContainerId, cancellationToken);
        if (container is null || container.IsSystem || container.DeploymentId is not null || container.StackId is not null)
            return;

        var platform = await uow.Platforms.GetByIdAsync(request.PlatformId, cancellationToken);
        if (platform is null) 
            return;

        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            Containers:
            [
                new ContainerAlertSnapshot(
                    Id: container.Id,
                    PlatformId: container.PlatformId,
                    ContainerId: container.DockerContainerId,
                    Name: container.Name,
                    PlatformName: platform.Name,
                    PlatformAddress: platform.Address)
            ]);

        await alertService.ProcessAsync(AlertType.UnmanagedContainerCreated, context, cancellationToken);
    }

    private static async Task ObserveCancellationAsync(Task task)
    {
        try { await task; }
        catch (OperationCanceledException) { }
    }
}

internal readonly record struct UnmanagedContainerAlertRequest(
    Guid PlatformId,
    string DockerContainerId);

internal readonly record struct UnmanagedContainerAlertKey(Guid PlatformId, string DockerContainerId);

internal readonly record struct ScheduledUnmanagedContainerAlert(
    UnmanagedContainerAlertRequest Request,
    DateTimeOffset DueAt)
{
    public UnmanagedContainerAlertKey Key { get; } = new(
        Request.PlatformId,
        Request.DockerContainerId.Trim().ToUpperInvariant());
}
