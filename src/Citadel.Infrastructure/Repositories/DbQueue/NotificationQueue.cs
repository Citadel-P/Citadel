using Domain.Contracts.Interfaces;
using System.Threading.Channels;

namespace Infrastructure.Repositories.DbQueue;

internal sealed class NotificationQueue : INotificationQueue
{
    private readonly Channel<INotificationWorkItem> _channel;

    public NotificationQueue()
    {
        _channel = Channel.CreateBounded<INotificationWorkItem>(new BoundedChannelOptions(1024)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait
        });
    }

    public ChannelReader<INotificationWorkItem> Reader => _channel.Reader;

    public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken cancellationToken)
    {
        if (!_channel.Writer.TryWrite(item))
        {
            return _channel.Writer.WriteAsync(item, cancellationToken);
        }
        return ValueTask.CompletedTask;
    }
}