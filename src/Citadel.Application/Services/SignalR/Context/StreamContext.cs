namespace Application.Services.SignalR.Context;

internal class StreamContext
{
    protected bool started;
    protected readonly Lock @lock = new();
    protected readonly HashSet<string> subscribers = [];
    public Task? StreamTask { get; set; }

    public virtual void AddSubscriber(string connectionId)
    {
        using (@lock.EnterScope())
        {
            subscribers.Add(connectionId);
        }
    }

    public virtual void RemoveSubscriber(string connectionId)
    {
        using (@lock.EnterScope())
        {
            subscribers.Remove(connectionId);
            if (subscribers.Count == 0)
                started = false;
        }
    }

    public bool IsEmpty
    {
        get
        {
            using (@lock.EnterScope())
                return subscribers.Count == 0;
        }
    }

    public bool TryStart()
    {
        using (@lock.EnterScope())
        {
            if (started) return false;
            started = true;
            return true;
        }
    }

    public void ResetStarted()
    {
        using (@lock.EnterScope())
        {
            started = false;
        }
    }
}
