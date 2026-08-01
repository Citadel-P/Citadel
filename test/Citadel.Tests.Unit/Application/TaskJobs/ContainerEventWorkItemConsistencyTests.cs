using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.Logging;
using Moq;
using System.Threading.Channels;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerEventWorkItemConsistencyTests
{
    [Fact]
    public async Task CreatedEvent_ShouldNotPublishCacheEntry_WhenCommitFails()
    {
        var platformId = Guid.CreateVersion7();
        const string containerId = "0123456789abcdef";
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.AddAsync(
                It.IsAny<Container>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var images = new Mock<IImageRepository>();
        images
            .Setup(repository => repository.GetByDockerImageIdAsync(
                It.IsAny<string>(),
                platformId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((Image?)null);
        var unitOfWork = CreateFailingUnitOfWork(containers.Object, images.Object);
        var cache = new Mock<IPlatformContainerCache>();
        var workItem = new ContainerCreatedWorkItem(
            new DaemonContainerEventInfo(
                "create",
                containerId,
                new DockerContainer(
                    "web",
                    "nginx:latest",
                    containerId,
                    "sha256:image",
                    ContainerStateStatus.Running)),
            platformId,
            Mock.Of<INotificationQueue>(),
            Channel.CreateUnbounded<UnmanagedContainerAlertRequest>().Writer,
            Mock.Of<IDockerDaemonStreamManager>(),
            cache.Object,
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<ILogger>());

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        cache.Verify(
            value => value.TryAddContainer(
                It.IsAny<Guid>(),
                It.IsAny<string>(),
                It.IsAny<Guid>()),
            Times.Never);
    }

    [Fact]
    public async Task DestroyedEvent_ShouldNotRemoveCacheEntry_WhenCommitFails()
    {
        var platformId = Guid.CreateVersion7();
        const string containerId = "fedcba9876543210";
        var container = new Container(
            "web",
            string.Empty,
            platformId,
            containerId,
            ContainerStateStatus.Running);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetByIdAsync(
                containerId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(container);
        containers
            .Setup(repository => repository.DeleteAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = CreateFailingUnitOfWork(containers.Object, Mock.Of<IImageRepository>());
        var cache = new Mock<IPlatformContainerCache>();
        var workItem = new ContainerDestroyedWorkItem(
            platformId,
            new DaemonContainerEventInfo("destroy", containerId, null),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IDockerDaemonStreamManager>(),
            cache.Object,
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<ILogger>());

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        cache.Verify(
            value => value.TryRemoveContainer(
                It.IsAny<Guid>(),
                It.IsAny<string>()),
            Times.Never);
    }

    private static Mock<IUnitOfWork> CreateFailingUnitOfWork(
        IContainerRepository containers,
        IImageRepository images)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Containers).Returns(containers);
        unitOfWork.SetupGet(work => work.Images).Returns(images);
        unitOfWork
            .Setup(work => work.CommitAsync(It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("commit failed"));
        return unitOfWork;
    }
}
