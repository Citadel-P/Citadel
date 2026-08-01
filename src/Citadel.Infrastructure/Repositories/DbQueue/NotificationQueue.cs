using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Hosting;
using System.Threading.Channels;

namespace Infrastructure.Repositories.DbQueue;

internal sealed class NotificationQueue : INotificationQueue, IDisposable
{
    private readonly Channel<INotificationWorkItem> _channel;
    private readonly CancellationTokenRegistration _stoppingRegistration;

    public NotificationQueue(IHostApplicationLifetime applicationLifetime)
    {
        _channel = Channel.CreateBounded<INotificationWorkItem>(new BoundedChannelOptions(1024)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait
        });
        _stoppingRegistration = applicationLifetime.ApplicationStopping.Register(Stop);
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

    public void Dispose()
    {
        _stoppingRegistration.Dispose();
        Stop();
    }

    private void Stop()
    {
        _channel.Writer.TryComplete();

        // The consumer has stopped; do not retain notifications that can no longer be sent.
        while (_channel.Reader.TryRead(out _))
        {
        }
    }
}
