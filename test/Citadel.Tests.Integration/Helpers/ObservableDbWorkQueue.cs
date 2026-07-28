using Domain.Contracts.Interfaces;
using System.Threading.Channels;

namespace Tests.Integration.Helpers;

internal sealed class ObservableDbWorkQueue : IDbWorkQueue
{
    private readonly Channel<IDbWorkItem> channel =
        Channel.CreateBounded<IDbWorkItem>(new BoundedChannelOptions(1024)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait
        });
    private readonly Lock stateLock = new();
    private readonly List<IdleWaiter> idleWaiters = [];
    private long sequence;
    private long lastCompletedSequence;
    private int pendingCount;

    public ChannelReader<IDbWorkItem> Reader => channel.Reader;

    public long CreateCheckpoint() => Interlocked.Read(ref sequence);

    public async ValueTask EnqueueAsync(
        IDbWorkItem item,
        CancellationToken cancellationToken)
    {
        var itemSequence = Interlocked.Increment(ref sequence);
        lock (stateLock)
        {
            pendingCount++;
        }

        try
        {
            await channel.Writer.WriteAsync(
                new ObservableDbWorkItem(item, itemSequence, Complete),
                cancellationToken);
        }
        catch
        {
            Complete(itemSequence);
            throw;
        }
    }

    public async ValueTask EnqueueAndWaitAsync(
        IDbWorkItem item,
        CancellationToken cancellationToken)
    {
        var awaitable = new AwaitableDbWorkItem(item);
        await EnqueueAsync(awaitable, cancellationToken);
        await awaitable.Completion.WaitAsync(cancellationToken);
    }

    public Task WaitForIdleAfterAsync(
        long checkpoint,
        CancellationToken cancellationToken)
    {
        lock (stateLock)
        {
            if (pendingCount == 0 && lastCompletedSequence > checkpoint)
                return Task.CompletedTask;

            var waiter = new IdleWaiter(checkpoint);
            idleWaiters.Add(waiter);
            return waiter.Completion.Task
                .WaitAsync(TimeSpan.FromSeconds(5), cancellationToken);
        }
    }

    private void Complete(long itemSequence)
    {
        List<TaskCompletionSource>? completedWaiters = null;

        lock (stateLock)
        {
            pendingCount--;
            lastCompletedSequence = Math.Max(lastCompletedSequence, itemSequence);

            if (pendingCount != 0)
                return;

            for (var index = idleWaiters.Count - 1; index >= 0; index--)
            {
                var waiter = idleWaiters[index];
                if (lastCompletedSequence <= waiter.Checkpoint)
                    continue;

                completedWaiters ??= [];
                completedWaiters.Add(waiter.Completion);
                idleWaiters.RemoveAt(index);
            }
        }

        if (completedWaiters is null)
            return;

        foreach (var waiter in completedWaiters)
            waiter.TrySetResult();
    }

    private sealed record IdleWaiter(long Checkpoint)
    {
        public TaskCompletionSource Completion { get; } =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
    }

    private sealed class ObservableDbWorkItem(
        IDbWorkItem inner,
        long sequence,
        Action<long> complete) : IDbWorkItem
    {
        public async Task ExecuteAsync(
            IUnitOfWork unitOfWork,
            CancellationToken cancellationToken)
        {
            try
            {
                await inner.ExecuteAsync(unitOfWork, cancellationToken);
            }
            finally
            {
                complete(sequence);
            }
        }
    }

    private sealed class AwaitableDbWorkItem(IDbWorkItem inner) : IDbWorkItem
    {
        private readonly TaskCompletionSource completion =
            new(TaskCreationOptions.RunContinuationsAsynchronously);

        public Task Completion => completion.Task;

        public async Task ExecuteAsync(
            IUnitOfWork unitOfWork,
            CancellationToken cancellationToken)
        {
            try
            {
                await inner.ExecuteAsync(unitOfWork, cancellationToken);
                completion.TrySetResult();
            }
            catch (Exception exception)
            {
                completion.TrySetException(exception);
                throw;
            }
        }
    }
}
