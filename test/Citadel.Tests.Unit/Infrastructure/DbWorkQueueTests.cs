using Domain.Contracts.Interfaces;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.Hosting;
using Moq;

namespace Tests.Unit.Infrastructure;

public sealed class DbWorkQueueTests
{
    [Fact]
    public async Task ApplicationStopping_CancelsPendingAwaitersAndDrainsQueue()
    {
        using var stopping = new CancellationTokenSource();
        var lifetime = new Mock<IHostApplicationLifetime>();
        lifetime.SetupGet(x => x.ApplicationStopping).Returns(stopping.Token);
        using var queue = new DbWorkQueue(lifetime.Object);

        var waiting = queue.EnqueueAndWaitAsync(
            new NoOpWorkItem(),
            CancellationToken.None).AsTask();

        stopping.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => waiting);
        Assert.False(queue.Reader.TryRead(out _));
        await queue.Reader.Completion;
    }

    private sealed class NoOpWorkItem : IDbWorkItem
    {
        public Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
            => Task.CompletedTask;
    }
}
