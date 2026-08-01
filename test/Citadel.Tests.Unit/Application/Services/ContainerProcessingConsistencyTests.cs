using Application.Features.Containers.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Deployments;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class ContainerProcessingConsistencyTests
{
    [Fact]
    public async Task RollbackProcessing_WhenContainerClaimWasLost_ShouldNotPublishStaleIdleState()
    {
        var actorId = Guid.CreateVersion7();
        var container = new Container(
            "web",
            "sha256:image",
            Guid.CreateVersion7(),
            "0123456789abcdef",
            ContainerStateStatus.Running);
        container.MarkProcessing(actorId);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.UpdateProcessingAsync(
                container.Id,
                ResourceControlState.Idle,
                null,
                container.RowVersion + 1,
                true,
                null,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Containers).Returns(containers.Object);
        unitOfWork.Setup(work => work.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var notificationQueue = new Mock<INotificationQueue>();
        var service = CreateContainerProcessingService(services, notificationQueue.Object);

        await service.RollbackProcessingAsync(
            new ProcessedResources([container], [], []),
            actorId,
            TestContext.Current.CancellationToken);

        notificationQueue.Verify(
            queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task RollbackProcessing_WhenDeploymentClaimWasLost_ShouldNotPublishStaleIdleState()
    {
        var actorId = Guid.CreateVersion7();
        var deployment = new Deployment(
            "web",
            actorId,
            Guid.CreateVersion7());
        deployment.MarkProcessing(actorId);

        var deployments = new Mock<IDeploymentRepository>();
        deployments
            .Setup(repository => repository.UpdateProcessingAsync(
                deployment.Id,
                DeploymentStatus.Pending,
                ResourceControlState.Idle,
                null,
                deployment.RowVersion + 1,
                true,
                null,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Deployments).Returns(deployments.Object);
        unitOfWork.Setup(work => work.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var notificationQueue = new Mock<INotificationQueue>();
        var service = new DeploymentProcessingService(
            services.GetRequiredService<IServiceScopeFactory>(),
            notificationQueue.Object,
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IHostApplicationLifetime>(),
            NullLogger<DeploymentProcessingService>.Instance);

        await service.RollbackProcessingAsync(
            [deployment],
            TestContext.Current.CancellationToken);

        notificationQueue.Verify(
            queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task MarkProcessing_ForDeploymentDelete_ShouldClaimContainerWithoutReclaimingParent()
    {
        var actorId = Guid.CreateVersion7();
        var deploymentId = Guid.CreateVersion7();
        var container = new Container(
            "web",
            "sha256:image",
            Guid.CreateVersion7(),
            "0123456789abcdef",
            ContainerStateStatus.Running,
            deploymentId: deploymentId);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetByIdAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        containers
            .Setup(repository => repository.UpdateProcessingAsync(
                container.Id,
                ResourceControlState.Processing,
                It.IsAny<long?>(),
                container.RowVersion,
                true,
                actorId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Containers).Returns(containers.Object);
        unitOfWork.Setup(work => work.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var service = new ContainerProcessingService(
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IDockerDaemonStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IPlatformContainerCache>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<IDbWorkQueue>(),
            Mock.Of<IHostApplicationLifetime>(),
            NullLogger<ContainerProcessingService>.Instance);

        var result = await service.MarkProcessingAsync(
            [container.Id],
            actorId,
            TestContext.Current.CancellationToken,
            claimParentResources: false);

        Assert.False(result.HasConflict);
        Assert.Single(result.Containers);
        Assert.Empty(result.Deployments);
        unitOfWork.VerifyGet(work => work.Deployments, Times.Never);
    }

    [Fact]
    public async Task CommandCompletion_ShouldNotWriteState_WhenClaimReleaseLosesRace()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        const string dockerContainerId = "0123456789abcdef";
        var claimed = new Container(
            "web",
            "sha256:image",
            platformId,
            dockerContainerId,
            ContainerStateStatus.Exited);
        var current = Container.FromPersistence(
            claimed.Id,
            platformId,
            dockerContainerId,
            "sha256:image",
            "web",
            created: 1,
            updated: 1,
            rowVersion: claimed.RowVersion + 1,
            controlStartedAt: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            controlTriggeredBy: actorId,
            controlState: ResourceControlState.Processing,
            state: ContainerStateStatus.Exited,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>());
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetContainerInfoAsync(
                dockerContainerId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(current);
        containers
            .Setup(repository => repository.UpdateProcessingAsync(
                current.Id,
                ResourceControlState.Idle,
                null,
                claimed.RowVersion + 1,
                true,
                null,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Containers).Returns(containers.Object);
        var notificationQueue = new Mock<INotificationQueue>();
        var workItem = new CompleteContainerCommandWorkItem(
            new ProcessedResources([claimed], [], []),
            new Dictionary<string, ContainerStateStatus>
            {
                [dockerContainerId] = ContainerStateStatus.Running
            },
            actorId,
            notificationQueue.Object,
            Mock.Of<IDockerDaemonStreamManager>(),
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            NullLogger<ContainerProcessingService>.Instance);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(ContainerStateStatus.Exited, current.State);
        containers.Verify(
            repository => repository.UpdateAsync(
                It.IsAny<Container>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        notificationQueue.Verify(
            queue => queue.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task StaleContainerDelete_ShouldNotEvictCache_WhenCommitFails()
    {
        var platformId = Guid.CreateVersion7();
        const string dockerContainerId = "0123456789abcdef";
        var container = new Container(
            "web",
            string.Empty,
            platformId,
            dockerContainerId,
            ContainerStateStatus.Offline);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetStaleByDockerIdsAsync(
                It.IsAny<string[]>(),
                It.IsAny<Guid[]>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        containers
            .Setup(repository => repository.DeleteAsync(
                It.IsAny<IEnumerable<Guid>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Containers).Returns(containers.Object);
        unitOfWork
            .Setup(work => work.CommitAsync(It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("commit failed"));
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var cache = new Mock<IPlatformContainerCache>();
        var emptyCacheEntries = new List<PlatformCacheEntry>();
        cache
            .Setup(value => value.TryGetPlatformsWithContainers(
                It.IsAny<string[]>(),
                out emptyCacheEntries))
            .Returns(false);
        var service = new ContainerProcessingService(
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<INotificationQueue>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IDockerDaemonStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            cache.Object,
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<IDbWorkQueue>(),
            Mock.Of<IHostApplicationLifetime>(),
            NullLogger<ContainerProcessingService>.Instance);

        await Assert.ThrowsAsync<InvalidOperationException>(() => service.DeleteContainers(
            new DeleteContainers([dockerContainerId]),
            Guid.CreateVersion7(),
            TestContext.Current.CancellationToken));

        cache.Verify(
            value => value.TryRemoveContainer(platformId, dockerContainerId),
            Times.Never);
    }

    private static ContainerProcessingService CreateContainerProcessingService(
        ServiceProvider services,
        INotificationQueue notificationQueue)
    {
        return new ContainerProcessingService(
            services.GetRequiredService<IServiceScopeFactory>(),
            notificationQueue,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IDockerDaemonStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IPlatformContainerCache>(),
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IContainerEventBroadcaster>(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<IDbWorkQueue>(),
            Mock.Of<IHostApplicationLifetime>(),
            NullLogger<ContainerProcessingService>.Instance);
    }
}
