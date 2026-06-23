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

    public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
    {
        var awaitable = new AwaitableDbWorkItem(item);
        await EnqueueAsync(awaitable, cancellationToken);
        await awaitable.WaitAsync(cancellationToken);
    }

    private sealed class AwaitableDbWorkItem(IDbWorkItem inner) : IDbWorkItem
    {
        private readonly TaskCompletionSource _completion = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            try
            {
                await inner.ExecuteAsync(uow, cancellationToken);
                _completion.TrySetResult();
            }
            catch (Exception ex)
            {
                _completion.TrySetException(ex);
                throw;
            }
        }

        public Task WaitAsync(CancellationToken cancellationToken)
            => _completion.Task.WaitAsync(cancellationToken);
    }
}
