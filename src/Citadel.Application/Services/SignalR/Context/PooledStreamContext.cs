using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class PooledStreamContext<T> : StreamContext where T : class
{
    public Channel<T> Channel { get; } = System.Threading.Channels.Channel.CreateBounded<T>(ApplicationModule.ChannelDefaultOptions());
    public CancellationTokenSource Cancellation { get; private set; } = new();

    public override void RemoveSubscriber(string connectionId)
    {
        Task? toObserve = null;
        lock (@lock)
        {
            subscribers.Remove(connectionId);
            if (IsEmpty)
            {
                try { Cancellation.Cancel(); } catch { }
                Channel.Writer.TryComplete();
                started = false;
                toObserve = StreamTask;
                StreamTask = null;
                try { Cancellation.Dispose(); } catch { }
                Cancellation = new CancellationTokenSource();
            }
        }
        if (toObserve != null)
            _ = toObserve.ContinueWith(t => { t.Dispose(); }, TaskContinuationOptions.ExecuteSynchronously);
    }
}