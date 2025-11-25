using Domain.Contracts.Interfaces;
using System.Threading.Channels;

namespace Infrastructure.Repositories.DbQueue;

internal sealed class DbWorkQueue : IDbWorkQueue
{
    private readonly Channel<IDbWorkItem> _channel;

    public DbWorkQueue()
    {
        _channel = Channel.CreateBounded<IDbWorkItem>(new BoundedChannelOptions(1024)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait
        });
    }

    public ChannelReader<IDbWorkItem> Reader => _channel.Reader;

    public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
    {
        if (!_channel.Writer.TryWrite(item))
        {
            return _channel.Writer.WriteAsync(item, cancellationToken);
        }
        return ValueTask.CompletedTask;
    }
}
