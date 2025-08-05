using System.IO;
using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class LogStreamContext(string containerId, int maxBufferSize = 50)
{
    public string ContainerId => containerId;
    public OneTimeFlag Started { get; } = new();
    public CancellationTokenSource Cancellation { get; } = new();
    public Channel<string> Channel { get; }
        = System.Threading.Channels.Channel.CreateBounded<string>(ApplicationModule.ChannelDefaultOptions());

    private readonly Lock @lock = new();
    private readonly HashSet<string> subscribers = [];
    private readonly Queue<string> buffer = new(maxBufferSize);

    public int MaxBufferSize { get; }

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

    public void Clear()
    {
        lock (@lock)
        {
            buffer.Clear();
            subscribers.Clear();
        }

        while (Channel.Reader.TryRead(out _)) { }
        Channel.Writer.TryComplete();
    }

    public bool IsEmpty
    {
        get
        {
            lock (@lock)
                return subscribers.Count == 0;
        }
    }

    public List<string> SubscribersSnapshot()
    {
        lock (@lock)
            return [.. subscribers];
    }

    public void AddToBuffer(string line)
    {
        lock (@lock)
        {
            if (MaxBufferSize > 0 && buffer.Count >= MaxBufferSize)
            {
                if (buffer.Count > 0)
                    buffer.Dequeue();
            }

            buffer.Enqueue(line);
        }
    }

    public string[] GetBufferedLogs()
    {
        lock (@lock)
            return [.. buffer];
    }

    internal sealed class OneTimeFlag
    {
        private int _value = 0;
        public bool TrySet() => Interlocked.Exchange(ref _value, 1) == 0;
    }
}
