using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class StatsStreamManagerTests
{
    private readonly Mock<IApplicationHubDispatcher> _dispatcher = new();

    [Fact]
    public void ContainerManager_ShouldReportSubscribersOnlyForTheRequestedPlatform()
    {
        var manager = new ContainerStreamManager(_dispatcher.Object);
        var subscribedPlatformId = Guid.CreateVersion7();
        var otherPlatformId = Guid.CreateVersion7();

        manager.AddSubscriber(
            Constants.WellKnownSignalRGroups.ContainersGroup(subscribedPlatformId),
            "connection-1");

        Assert.True(manager.HasStatsSubscribers(subscribedPlatformId));
        Assert.False(manager.HasStatsSubscribers(otherPlatformId));
    }

    [Fact]
    public void PlatformManager_ShouldReportStatsSubscribersForThePlatformsGroup()
    {
        var manager = new PlatformStreamManager(_dispatcher.Object, Mock.Of<IServiceScopeFactory>());

        Assert.False(manager.HasStatsSubscribers);

        manager.AddSubscriber(Constants.WellKnownSignalRGroups.PlatformsGroup, "connection-1");

        Assert.True(manager.HasStatsSubscribers);
    }

    [Fact]
    public async Task DeploymentManager_ShouldRefreshPlatformSummaryWithoutDeploymentSubscribers()
    {
        var platformStreamManager = new Mock<IPlatformStreamManager>();
        var manager = new DeploymentStreamManager(_dispatcher.Object, platformStreamManager.Object);
        var platformId = Guid.CreateVersion7();
        var deployment = new Deployment("deployment", Constants.SystemId, platformId);

        await manager.SendDeploymentInfo(deployment);

        platformStreamManager.Verify(stream => stream.RefreshPlatform(platformId), Times.Once);
        _dispatcher.Verify(
            dispatcher => dispatcher.SendDeploymentInfo(It.IsAny<Deployment>(), It.IsAny<string>()),
            Times.Never);
    }

    [Fact]
    public async Task PlatformManager_ShouldReloadAndBroadcastPlatformSummary()
    {
        var platform = new Platform(
            "platform",
            "unix:///var/run/docker.sock",
            0,
            0,
            0,
            1,
            1024,
            "1.0.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
        var repository = new Mock<IPlatformRepository>();
        repository
            .Setup(candidate => candidate.GetPlatformWithLatestStatAsync(platform.Id, CancellationToken.None))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(candidate => candidate.Platforms).Returns(repository.Object);
        var serviceProvider = new Mock<IServiceProvider>();
        serviceProvider
            .Setup(candidate => candidate.GetService(typeof(IUnitOfWork)))
            .Returns(unitOfWork.Object);
        var scope = new Mock<IServiceScope>();
        scope.SetupGet(candidate => candidate.ServiceProvider).Returns(serviceProvider.Object);
        var scopeFactory = new Mock<IServiceScopeFactory>();
        scopeFactory.Setup(candidate => candidate.CreateScope()).Returns(scope.Object);
        var manager = new PlatformStreamManager(_dispatcher.Object, scopeFactory.Object);
        manager.AddSubscriber(Constants.WellKnownSignalRGroups.PlatformsGroup, "connection-1");

        await manager.RefreshPlatform(platform.Id);

        _dispatcher.Verify(candidate => candidate.PushPlatformUpdate(platform), Times.Once);
    }

    [Fact]
    public async Task StackManager_ShouldRefreshPlatformSummaryWithoutStackSubscribers()
    {
        var platformStreamManager = new Mock<IPlatformStreamManager>();
        var manager = new StackStreamManager(_dispatcher.Object, platformStreamManager.Object);
        var platformId = Guid.CreateVersion7();
        var stack = Stack.Create(
            "stack",
            Constants.SystemId,
            StackSource.WebEditor,
            platformId,
            new ManualStack("services: {}", StackUpdateBehavior.Notify));

        await manager.SendStackInfo(stack);

        platformStreamManager.Verify(stream => stream.RefreshPlatform(platformId), Times.Once);
        _dispatcher.Verify(
            dispatcher => dispatcher.SendStackInfo(It.IsAny<Stack>(), It.IsAny<string>()),
            Times.Never);
    }
}
