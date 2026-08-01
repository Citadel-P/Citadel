using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Hosting;
using System.Threading.Channels;

namespace Infrastructure.Repositories.DbQueue;

internal sealed class DbWorkQueue : IDbWorkQueue, IDisposable
{
    private readonly Channel<IDbWorkItem> _channel;
    private readonly HashSet<AwaitableDbWorkItem> _pending = [];
    private readonly Lock _gate = new();
    private readonly CancellationTokenRegistration _stoppingRegistration;

    public DbWorkQueue(IHostApplicationLifetime applicationLifetime)
    {
        _channel = Channel.CreateBounded<IDbWorkItem>(new BoundedChannelOptions(1024)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait
        });
        _stoppingRegistration = applicationLifetime.ApplicationStopping.Register(Stop);
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
        using (_gate.EnterScope())
            _pending.Add(awaitable);

        try
        {
            await EnqueueAsync(awaitable, cancellationToken);
            await awaitable.WaitAsync(cancellationToken);
        }
        finally
        {
            using (_gate.EnterScope())
                _pending.Remove(awaitable);
        }
    }

    public void Dispose()
    {
        _stoppingRegistration.Dispose();
        Stop();
    }

    private void Stop()
    {
        _channel.Writer.TryComplete();

        AwaitableDbWorkItem[] pending;
        using (_gate.EnterScope())
            pending = [.. _pending];

        foreach (var item in pending)
            item.Cancel();

        // Do not retain ordinary fire-and-forget work after its only consumer has stopped.
        while (_channel.Reader.TryRead(out _))
        {
        }
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

        public void Cancel()
            => _completion.TrySetCanceled();
    }
}
