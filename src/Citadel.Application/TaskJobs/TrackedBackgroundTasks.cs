namespace Application.TaskJobs;

internal sealed class TrackedBackgroundTasks
{
    private readonly HashSet<Task> tasks = [];
    private readonly Lock gate = new();

    public void Add(Task task)
    {
        using (gate.EnterScope())
            tasks.Add(task);

        _ = task.ContinueWith(
            completed =>
            {
                using (gate.EnterScope())
                    tasks.Remove(completed);
            },
            CancellationToken.None,
            TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);
    }

    public async Task WaitForCompletionOrDelayAsync(
        TimeSpan delay,
        CancellationToken cancellationToken)
    {
        Task[] snapshot;
        using (gate.EnterScope())
            snapshot = tasks.ToArray();

        if (snapshot.Length == 0)
        {
            await Task.Delay(delay, cancellationToken);
            return;
        }

        try
        {
            await Task.WhenAny(snapshot).WaitAsync(delay, cancellationToken);
        }
        catch (TimeoutException)
        {
        }
    }

    public async Task DrainAsync(CancellationToken cancellationToken = default)
    {
        while (true)
        {
            Task[] snapshot;
            using (gate.EnterScope())
                snapshot = tasks.ToArray();

            if (snapshot.Length == 0)
                return;

            await Task.WhenAll(snapshot).WaitAsync(cancellationToken);
        }
    }
}

internal static class WorkerPollingDelay
{
    private static readonly TimeSpan MaximumIdleDelay = TimeSpan.FromSeconds(10);

    public static TimeSpan Next(TimeSpan current, TimeSpan minimum, bool dispatchedWork)
    {
        if (dispatchedWork)
            return minimum;

        return TimeSpan.FromSeconds(
            Math.Min(
                MaximumIdleDelay.TotalSeconds,
                Math.Max(minimum.TotalSeconds, current.TotalSeconds * 2)));
    }
}
