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

    /// <summary>
    /// Releases and removes all barriers owned by a deleted platform.
    /// </summary>
    void RemovePlatform(Guid platformId);
}

internal sealed class SyncBarrier : ISyncBarrier
{
    private static readonly TimeSpan RemovedPlatformRetention = TimeSpan.FromMinutes(15);
    private const int MaximumRemovedPlatforms = 4096;
    private readonly ConcurrentDictionary<(Type Job, Guid PlatformId), TaskCompletionSource> _barriers = new();
    private readonly ConcurrentDictionary<Guid, DateTimeOffset> _removedPlatforms = new();

    internal int Count => _barriers.Count;

    public ValueTask WaitForAsync<TJob>(Guid platformId, CancellationToken cancellationToken = default)
        => WaitForAsync(typeof(TJob), platformId, cancellationToken);

    public void MarkSynced<TJob>(Guid platformId)
        => MarkSynced(typeof(TJob), platformId);

    public void RemovePlatform(Guid platformId)
    {
        _removedPlatforms[platformId] = DateTimeOffset.UtcNow.Add(RemovedPlatformRetention);

        foreach (var entry in _barriers)
        {
            if (entry.Key.PlatformId != platformId || !_barriers.TryRemove(entry.Key, out var barrier))
                continue;

            // A job that was already waiting for this platform must not remain rooted forever
            // after the platform is deleted. Completing it lets the job perform its normal
            // existence check and exit.
            barrier.TrySetResult();
        }

        CompactRemovedPlatforms();
    }

    private ValueTask WaitForAsync(Type jobType, Guid platformId, CancellationToken cancellationToken)
    {
        if (IsRecentlyRemoved(platformId))
            return ValueTask.CompletedTask;

        var key = (jobType, platformId);

        var tcs = _barriers.GetOrAdd(
            key,
            _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));

        // Close the race where platform deletion happens between the first tombstone
        // check and barrier creation.
        if (IsRecentlyRemoved(platformId))
        {
            if (_barriers.TryRemove(key, out var removed))
                removed.TrySetResult();
            return ValueTask.CompletedTask;
        }

        // If already completed, return a completed ValueTask
        if (tcs.Task.IsCompleted)
            return ValueTask.CompletedTask;

        return cancellationToken.CanBeCanceled
            ? new ValueTask(tcs.Task.WaitAsync(cancellationToken))
            : new ValueTask(tcs.Task);
    }

    private void MarkSynced(Type jobType, Guid platformId)
    {
        if (IsRecentlyRemoved(platformId))
            return;

        var key = (jobType, platformId);

        // Get or create the TCS and mark it completed
        var tcs = _barriers.GetOrAdd(
            key,
            _ => new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously));

        // Close the matching race where deletion happens after the tombstone check but
        // before this completed barrier is inserted. Otherwise one entry can survive for
        // every job type that finishes while its platform is being deleted.
        if (IsRecentlyRemoved(platformId))
        {
            if (_barriers.TryRemove(key, out var removed))
                removed.TrySetResult();
            return;
        }

        tcs.TrySetResult(); // don't remove; keep so future WaitForAsync sees completed
    }

    private bool IsRecentlyRemoved(Guid platformId)
    {
        if (!_removedPlatforms.TryGetValue(platformId, out var expiresAt))
            return false;

        if (expiresAt > DateTimeOffset.UtcNow)
            return true;

        _removedPlatforms.TryRemove(
            new KeyValuePair<Guid, DateTimeOffset>(platformId, expiresAt));
        return false;
    }

    private void CompactRemovedPlatforms()
    {
        if (_removedPlatforms.Count <= MaximumRemovedPlatforms)
            return;

        var removeCount = _removedPlatforms.Count - MaximumRemovedPlatforms;
        foreach (var entry in _removedPlatforms
                     .OrderBy(static entry => entry.Value)
                     .Take(removeCount))
        {
            _removedPlatforms.TryRemove(entry);
        }
    }
}
