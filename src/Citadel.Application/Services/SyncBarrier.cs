using System.Collections.Concurrent;

/// <summary>
/// A type-safe, asynchronous synchronization barrier for coordinating background jobs or services.
/// Each barrier is keyed by a <see cref="Type"/> and a platform ID, allowing other jobs to wait 
/// until a specific job has completed its first run or reached a defined ready state.
/// </summary>
internal interface ISyncBarrier
{
    /// <summary>
    /// Waits for the specified job type to mark its barrier.
    /// </summary>
    ValueTask WaitForAsync<TJob>(Guid platformId, CancellationToken ct = default);
    /// <summary>
    /// Marks the barrier for the specified job type as completed.
    /// </summary>
    void MarkSynced<TJob>(Guid platformId);
}

internal sealed class SyncBarrier : ISyncBarrier
{
    private readonly ConcurrentDictionary<(Type Job, Guid PlatformId), TaskCompletionSource> _barriers = new();

    public ValueTask WaitForAsync<TJob>(Guid platformId, CancellationToken cancellationToken = default)
        => WaitForAsync(typeof(TJob), platformId, cancellationToken);

    public void MarkSynced<TJob>(Guid platformId)
        => MarkSynced(typeof(TJob), platformId);

    private ValueTask WaitForAsync(Type jobType, Guid platformId, CancellationToken cancellationToken)
    {
        var key = (jobType, platformId);

        var tcs = _barriers.GetOrAdd(
            key,
            _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));

        // If already completed, return a completed ValueTask
        if (tcs.Task.IsCompleted)
            return ValueTask.CompletedTask;

        return cancellationToken.CanBeCanceled
            ? new ValueTask(tcs.Task.WaitAsync(cancellationToken))
            : new ValueTask(tcs.Task);
    }

    private void MarkSynced(Type jobType, Guid platformId)
    {
        var key = (jobType, platformId);

        // Get or create the TCS and mark it completed
        var tcs = _barriers.GetOrAdd(
            key,
            _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));

        tcs.TrySetResult(); // don't remove; keep so future WaitForAsync sees completed
    }
}