namespace Application.Services.SignalR.Context;

internal class StreamContext
{
    protected bool started;
    protected readonly Lock @lock = new();
    protected readonly HashSet<string> subscribers = [];
    public Task? StreamTask { get; set; }

    public virtual void AddSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Add(connectionId);
        }
    }

    public virtual void RemoveSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Remove(connectionId);
            if (IsEmpty)
            {
                started = false;
            }
        }
    }

    public bool IsEmpty
    {
        get
        {
            lock (@lock)
                return subscribers.Count == 0;
        }
    }

    public bool TryStart()
    {
        lock (@lock)
        {
            if (started) return false;
            started = true;
            return true;
        }
    }
}