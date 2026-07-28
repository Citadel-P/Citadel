using Application.Services.SignalR;
using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerDependentResourceSynchronizerTests
{
    [Fact]
    public async Task SynchronizeAsync_WhenDeploymentSyncFails_StillSynchronizesStacks()
    {
        var platformId = Guid.CreateVersion7();
        var deployments = new Mock<IDeploymentRepository>();
        deployments
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("deployment sync failed"));
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetInfoAsync(
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                platformId))
            .ReturnsAsync(Array.Empty<Stack>());
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Deployments).Returns(deployments.Object);
        unitOfWork.SetupGet(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.RollbackAsync()).Returns(Task.CompletedTask);

        await ContainerDependentResourceSynchronizer.SynchronizeAsync(
            unitOfWork.Object,
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<INotificationQueue>(),
            platformId,
            isOnline: true,
            logger: NullLogger.Instance,
            cancellationToken: TestContext.Current.CancellationToken);

        unitOfWork.Verify(x => x.RollbackAsync(), Times.Once);
        stacks.Verify(x => x.GetInfoAsync(
            It.IsAny<CancellationToken>(),
            It.IsAny<IReadOnlyCollection<Guid>?>(),
            platformId), Times.Once);
    }
}
