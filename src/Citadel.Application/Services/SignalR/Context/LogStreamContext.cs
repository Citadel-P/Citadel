using System.Threading.Channels;

namespace Application.Services.SignalR.Context;

internal sealed class LogStreamContext : StreamContext
{
    public Channel<string> Channel { get; } = System.Threading.Channels.Channel.CreateBounded<string>(ApplicationModule.ChannelDefaultOptions());
    public CancellationTokenSource Cancellation { get; } = new();

    private readonly LimitedQueue<string> buffer = new(50);


    public override void RemoveSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Remove(connectionId);
            if (IsEmpty)
            {
                buffer.Clear();
                Cancellation.Cancel();
                Channel.Writer.TryComplete();
                started = false;
            }
        }
    }

    public void AddToBuffer(string log)
    {
        buffer.Enqueue(log);
    }

    public IReadOnlyCollection<string> GetBufferedLogs() => buffer.ToArray();
}

internal sealed class LimitedQueue<T>(int capacity)
{
    private readonly Queue<T> queue = new(capacity);
    private readonly int capacity = capacity;
    private readonly Lock @lock = new();

    public void Enqueue(T item)
    {
        lock (@lock)
        {
            if (capacity <= 0) return;
            if (queue.Count >= capacity)
                queue.Dequeue();
            queue.Enqueue(item);
        }
    }

    public T[] ToArray()
    {
        lock (@lock)
        {
            return [.. queue];
        }
    }

    public void Clear()
    {
        lock (@lock)
        {
            queue.Clear();
        }
    }
}