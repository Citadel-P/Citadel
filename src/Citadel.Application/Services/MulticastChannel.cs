using System.Threading.Channels;

namespace Application.Services;

internal class MulticastChannel<T>
{
    protected readonly List<Channel<T>> subscribers = [];
    private readonly Lock @lock = new();

    public ChannelReader<T> AddSubscriber()
    {
        lock (@lock)
        {
            var channel = Channel.CreateBounded<T>(ApplicationModule.ChannelDefaultOptions());
            subscribers.Add(channel);
            return channel.Reader;
        }
    }

    public async ValueTask PublishAsync(T @event, CancellationToken cancellationToken)
    {
            foreach (var sub in subscribers)
                await sub.Writer.WriteAsync(@event, cancellationToken);
    }

    public void Complete()
    {
        lock (@lock)
            foreach (var sub in subscribers)
                sub.Writer.TryComplete();
    }

    public void RemoveSubscriber(Channel<T> channel)
    {
        lock (@lock) subscribers.Remove(channel);
    }
}
