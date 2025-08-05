using System.Threading.Channels;
using Hosting.Common.ObjectPoolManager;

namespace Application.Services.SignalR.Context;

internal class StreamContext<T> where T : class
{
    public CancellationTokenSource Cancellation { get; } = new();
    public Channel<PooledHandle<T>> Channel { get; } 
        = System.Threading.Channels.Channel.CreateBounded<PooledHandle<T>>(ApplicationModule.ChannelDefaultOptions());
    
    private readonly Lock @lock = new();
    private readonly HashSet<string> subscribers = [];

    public void AddSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Add(connectionId);
        }
    }

    public void RemoveSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Remove(connectionId);
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
}