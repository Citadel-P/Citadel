using System.Threading.Channels;
using Hosting.Common.ObjectPoolManager;

namespace Application.Services.SignalR.Context;

internal class PooledStreamContext<T> : StreamContext where T : class
{
    public CancellationTokenSource Cancellation { get; } = new();
    public Channel<PooledHandle<T>> Channel { get; }
        = System.Threading.Channels.Channel.CreateBounded<PooledHandle<T>>(ApplicationModule.ChannelDefaultOptions());

    public override void RemoveSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Remove(connectionId);
            if (IsEmpty)
            {
                Channel.Writer.TryComplete();
                Cancellation.Cancel();
                started = false;
            }
        }
    }
}