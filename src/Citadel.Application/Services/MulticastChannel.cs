using Hosting.Common;
using System.Threading.Channels;

namespace Application.Services;

internal class MulticastChannel<T>
{
    private readonly List<Channel<T>> subscribers = [];
    private readonly Lock @lock = new();

    // Add a subscriber
    public ChannelReader<T> AddSubscriber()
    {
        var channel = Channel.CreateBounded<T>(Helpers.ChannelDefaultOptions());
        lock (@lock)
        {
            subscribers.Add(channel);
        }
        return channel.Reader;
    }

    // Remove a subscriber by Channel
    public void RemoveSubscriber(Channel<T> channel)
    {
        lock (@lock)
        {
            subscribers.Remove(channel);
            channel.Writer.TryComplete();
        }
    }

    // Remove a subscriber by ChannelReader
    public void RemoveSubscriber(ChannelReader<T> reader)
    {
        lock (@lock)
        {
            var channel = subscribers.FirstOrDefault(c => c.Reader == reader);
            if (channel != null)
            {
                subscribers.Remove(channel);
                channel.Writer.TryComplete();
            }
        }
    }

    // Publish an event to all subscribers
    public async ValueTask PublishAsync(T @event, CancellationToken cancellationToken = default)
    {
        Channel<T>[] snapshot;

        // Take a snapshot under lock to avoid holding the lock during async operations
        lock (@lock)
        {
            snapshot = [.. subscribers];
        }

        // Publish outside the lock
        foreach (var sub in snapshot)
        {
            try
            {
                await sub.Writer.WriteAsync(@event, cancellationToken).ConfigureAwait(false);
            }
            catch (ChannelClosedException)
            {
                // Already removed → ignore
            }
        }
    }

    // Complete all subscribers
    public void Complete()
    {
        Channel<T>[] snapshot;
        lock (@lock)
        {
            snapshot = [.. subscribers];
            subscribers.Clear();
        }

        foreach (var sub in snapshot)
            sub.Writer.TryComplete();
    }
}
