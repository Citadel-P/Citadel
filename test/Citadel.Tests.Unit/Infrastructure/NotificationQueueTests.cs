using Domain.Contracts.Interfaces;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Threading.Channels;

namespace Tests.Unit.Infrastructure;

public sealed class NotificationQueueTests
{
    [Fact]
    public async Task ApplicationStopping_ClosesPendingWriterAndDrainsQueue()
    {
        using var stopping = new CancellationTokenSource();
        var lifetime = new Mock<IHostApplicationLifetime>();
        lifetime.SetupGet(value => value.ApplicationStopping).Returns(stopping.Token);
        using var queue = new NotificationQueue(lifetime.Object);

        for (var index = 0; index < 1024; index++)
            await queue.EnqueueAsync(new NoOpNotification(), TestContext.Current.CancellationToken);

        var pendingWrite = queue.EnqueueAsync(
            new NoOpNotification(),
            CancellationToken.None).AsTask();

        Assert.False(pendingWrite.IsCompleted);
        stopping.Cancel();

        await Assert.ThrowsAsync<ChannelClosedException>(() => pendingWrite);
        Assert.False(queue.Reader.TryRead(out _));
        await queue.Reader.Completion;
    }

    private sealed class NoOpNotification : INotificationWorkItem
    {
        public Task ExecuteAsync(CancellationToken cancellationToken)
            => Task.CompletedTask;
    }
}
