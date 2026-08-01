using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Moq;
using System.Threading.Channels;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ImageSyncWorkItemConsistencyTests
{
    [Fact]
    public async Task CommittedImageSync_ReleasesBarrier_WhenNotificationQueueFails()
    {
        var platformId = Guid.CreateVersion7();
        var images = new Mock<IImageRepository>();
        images
            .Setup(repository => repository.GetByPlatformIdAsync(
                platformId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        images
            .Setup(repository => repository.BulkUpsertAsync(
                It.IsAny<IEnumerable<Image>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Images).Returns(images.Object);
        unitOfWork
            .Setup(work => work.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        var notifications = new Mock<INotificationQueue>();
        notifications
            .Setup(queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.FromException(new ChannelClosedException()));
        var barrier = new Mock<ISyncBarrier>();
        var workItem = new ImageSyncWorkItem(
            [],
            Mock.Of<IImageStreamManager>(),
            notifications.Object,
            barrier.Object,
            new PlatformHealth(
                platformId,
                "unix:///var/run/docker.sock",
                PlatformConnectorType.Local,
                true));

        await Assert.ThrowsAsync<ChannelClosedException>(() => workItem.ExecuteAsync(
            unitOfWork.Object,
            TestContext.Current.CancellationToken));

        barrier.Verify(
            value => value.MarkSynced<ImageSyncJob>(platformId),
            Times.Once);
    }
}
