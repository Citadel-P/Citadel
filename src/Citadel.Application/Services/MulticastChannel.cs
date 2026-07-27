using Hosting.Common;
using System.Threading.Channels;

namespace Application.Services;

internal class MulticastChannel<T>
{
    private readonly List<Channel<T>> subscribers = [];
    private readonly Lock @lock = new();
    private Channel<T>[] subscriberSnapshot = [];
    private bool completed;

    public bool HasSubscribers => Volatile.Read(ref subscriberSnapshot).Length != 0;

    public ChannelReader<T> AddSubscriber()
    {
        var channel = Channel.CreateBounded<T>(Helpers.ChannelDefaultOptions(
            singleWriter: false,
            boundedChannelFullMode: BoundedChannelFullMode.DropOldest));

        using (@lock.EnterScope())
        {
            if (completed)
            {
                channel.Writer.TryComplete();
                return channel.Reader;
            }

            subscribers.Add(channel);
            Volatile.Write(ref subscriberSnapshot, [.. subscribers]);
        }

        return channel.Reader;
    }

    public void RemoveSubscriber(Channel<T> channel)
    {
        using (@lock.EnterScope())
        {
            if (subscribers.Remove(channel))
                Volatile.Write(ref subscriberSnapshot, [.. subscribers]);

            channel.Writer.TryComplete();
        }
    }

    public void RemoveSubscriber(ChannelReader<T> reader)
    {
        using (@lock.EnterScope())
        {
            var channel = subscribers.FirstOrDefault(c => c.Reader == reader);
            if (channel != null)
            {
                subscribers.Remove(channel);
                Volatile.Write(ref subscriberSnapshot, [.. subscribers]);
                channel.Writer.TryComplete();
            }
        }
    }

    public ValueTask PublishAsync(T @event, CancellationToken cancellationToken = default)
    {
        foreach (var subscriber in Volatile.Read(ref subscriberSnapshot))
            subscriber.Writer.TryWrite(@event);

        return ValueTask.CompletedTask;
    }

    public void Complete()
    {
        Channel<T>[] snapshot;
        using (@lock.EnterScope())
        {
            if (completed)
                return;

            completed = true;
            snapshot = [.. subscribers];
            subscribers.Clear();
            Volatile.Write(ref subscriberSnapshot, []);
        }

        foreach (var sub in snapshot)
            sub.Writer.TryComplete();
    }
}
