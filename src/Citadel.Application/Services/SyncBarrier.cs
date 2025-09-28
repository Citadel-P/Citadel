using System.Collections.Concurrent;

namespace Application.Services;

/// <summary>
/// A type-safe, asynchronous synchronization barrier for coordinating background jobs or services.
/// Each barrier is keyed by a <see cref="Type"/> (typically the job type), allowing other jobs to wait 
/// until a specific job has completed its first run or reached a defined ready state.
/// </summary>
internal interface ISyncBarrier
{
    /// <summary>
    /// Waits for the specified job types to mark its barrier.
    /// </summary>
    ValueTask WaitForAllAsync<T1, T2>(CancellationToken cancellationToken = default);

    /// <summary>
    /// Waits for the specified job type to mark its barrier.
    /// </summary>
    ValueTask WaitForAsync<TJob>(CancellationToken cancellationToken = default);

    /// <summary>
    /// Marks the barrier for the specified job type as completed.
    /// </summary>
    void MarkSynced<TJob>();
}

internal sealed class SyncBarrier : ISyncBarrier
{
    private readonly ConcurrentDictionary<Type, TaskCompletionSource> _barriers = new();

    public ValueTask WaitForAllAsync<T1, T2>(CancellationToken cancellationToken = default) => WaitForAllAsync([typeof(T1), typeof(T2)], cancellationToken);
    public ValueTask WaitForAsync<TJob>(CancellationToken cancellationToken = default) => WaitForAsync(typeof(TJob), cancellationToken);
    public void MarkSynced<TJob>() => MarkSynced(typeof(TJob));

    private ValueTask WaitForAsync(Type jobType, CancellationToken cancellationToken = default)
    {
        var tcs = _barriers.GetOrAdd(jobType, _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));

        if (tcs.Task.IsCompleted)
            return ValueTask.CompletedTask;

        return cancellationToken.CanBeCanceled
            ? new ValueTask(tcs.Task.WaitAsync(cancellationToken))
            : new ValueTask(tcs.Task);
    }

    private void MarkSynced(Type jobType)
    {
        var tcs = _barriers.GetOrAdd(jobType, _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));
        tcs.TrySetResult();
    }

    private async ValueTask WaitForAllAsync(IEnumerable<Type> jobTypes, CancellationToken cancellationToken = default)
    {
        foreach (var type in jobTypes)
            await WaitForAsync(type, cancellationToken).ConfigureAwait(false);
    }
}