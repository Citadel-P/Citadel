using System.Threading.Channels;
using Hosting.Common;

namespace Application.Services.SignalR.Context;

internal sealed class ChannelStreamContext<T> : StreamContext, IDisposable where T : class
{
    public Channel<T> Channel { get; } = System.Threading.Channels.Channel.CreateBounded<T>(Helpers.ChannelDefaultOptions());
    public CancellationTokenSource Cancellation { get; } = new();
    private bool disposed;

    public override void RemoveSubscriber(string connectionId)
    {
        base.RemoveSubscriber(connectionId);
        if (IsEmpty)
            Dispose();
    }

    public void Dispose()
    {
        using (@lock.EnterScope())
        {
            if (disposed)
                return;

            disposed = true;
            started = false;
            StreamTask = null;
        }

        try { Cancellation.Cancel(); } catch { }
        Channel.Writer.TryComplete();
        Cancellation.Dispose();
    }
}
