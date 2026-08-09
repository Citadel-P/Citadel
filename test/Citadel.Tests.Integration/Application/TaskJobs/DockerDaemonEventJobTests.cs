using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Runtime.CompilerServices;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public sealed class DockerDaemonEventJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> platformConnectorFactory = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerConnectorFactory = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();
    private readonly Mock<IContainerConnector> containerConnector = new();
    private readonly Mock<ISwarmReconciliationCoordinator> swarmReconciliationCoordinator = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private TaskCompletionSource eventHandled = NewCompletionSource();
    private DaemonContainerEventInfo daemonEvent = CreateContainerEvent(
        ContainerStateStatus.Exited,
        isSwarmTask: true);
    private Guid platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddHostedService<DockerDaemonEventJob>();
        services.ReplaceService<IPlatformHealthBroadCaster>(broadcaster);
        services.ReplaceService<IConnectorFactory<IPlatformConnector>>(platformConnectorFactory.Object);
        services.ReplaceService<IConnectorFactory<IContainerConnector>>(containerConnectorFactory.Object);
        services.ReplaceService<ISwarmReconciliationCoordinator>(swarmReconciliationCoordinator.Object);

        platformConnectorFactory
            .Setup(factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(platformConnector.Object);
        containerConnectorFactory
            .Setup(factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(containerConnector.Object);
        platformConnector
            .Setup(connector => connector.StreamDaemonEventAsync(
                It.IsAny<StreamDaemonEventCommand>(),
                It.IsAny<CancellationToken>()))
            .Returns((StreamDaemonEventCommand _, CancellationToken cancellationToken) =>
                StreamEventAsync(cancellationToken));
        swarmReconciliationCoordinator
            .Setup(coordinator => coordinator.NotifyDaemonEventAsync(
                It.IsAny<Guid>(),
                It.IsAny<DaemonEventInfo>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        containerConnector
            .Setup(connector => connector.DeleteAsync(
                It.IsAny<DeleteContainerCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork unitOfWork)
    {
        var platform = Fakes.GetDummyPlatform();
        platform.PartialUpdate(
            descriptor: CreateSwarmDescriptor(),
            pruneHistoricalSwarmTaskContainers: true);
        await unitOfWork.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
    }

    [Fact]
    public async Task TerminalSwarmTaskEvent_ShouldPruneContainerImmediately_WhenEnabled()
    {
        await PublishPlatformOnlineAsync();
        await eventHandled.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        containerConnector.Verify(connector => connector.DeleteAsync(
            It.Is<DeleteContainerCommand>(command =>
                command.ContainerIds.SequenceEqual(new[] { daemonEvent.ContainerId })
                && command.PlatformAddress == "https://original.address"
                && command.Volume == false
                && command.Force == false
                && command.Link == false),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task TerminalSwarmTaskEvent_ShouldKeepContainer_WhenPruningDisabled()
    {
        await SetPruningEnabledAsync(false);

        await PublishPlatformOnlineAsync();
        await eventHandled.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        containerConnector.Verify(connector => connector.DeleteAsync(
            It.IsAny<DeleteContainerCommand>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Theory]
    [InlineData(ContainerStateStatus.Running, true)]
    [InlineData(ContainerStateStatus.Exited, false)]
    public async Task NonHistoricalContainerEvent_ShouldNotPruneContainer(
        ContainerStateStatus state,
        bool isSwarmTask)
    {
        daemonEvent = CreateContainerEvent(state, isSwarmTask);
        eventHandled = NewCompletionSource();

        await PublishPlatformOnlineAsync();
        await eventHandled.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        containerConnector.Verify(connector => connector.DeleteAsync(
            It.IsAny<DeleteContainerCommand>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    private async Task PublishPlatformOnlineAsync()
    {
        await broadcaster.PublishAsync(
            new PlatformHealth(
                platformId,
                "https://original.address",
                PlatformConnectorType.Agent,
                IsOnLine: true,
                IsValidated: true),
            TestContext.Current.CancellationToken);
    }

    private async Task SetPruningEnabledAsync(bool enabled)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetByIdAsync(
            platformId,
            TestContext.Current.CancellationToken);
        Assert.NotNull(platform);
        platform.PartialUpdate(pruneHistoricalSwarmTaskContainers: enabled);
        await unitOfWork.Platforms.UpdateAsync(platform, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
    }

    private async IAsyncEnumerable<DaemonEventInfo> StreamEventAsync(
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        yield return daemonEvent;
        eventHandled.TrySetResult();
        await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
    }

    private static DaemonContainerEventInfo CreateContainerEvent(
        ContainerStateStatus state,
        bool isSwarmTask)
    {
        const string containerId = "historical-task-container";
        return new DaemonContainerEventInfo(
            "die",
            containerId,
            new DockerContainer(
                Name: "/redis.1.old-task",
                Image: "redis:latest",
                Id: containerId,
                ImageId: "sha256:redis",
                State: state,
                IsSwarmTask: isSwarmTask));
    }

    private static DockerSwarmPlatformDescriptor CreateSwarmDescriptor()
        => new(
            NodeID: "manager-1",
            NodeAddr: "10.0.0.1",
            LocalNodeState: "active",
            ControlAvailable: true,
            Nodes: 1,
            Managers: 1,
            DaemonId: "daemon-1",
            ContainerCount: 1,
            ContainersRunning: 1,
            ContainersPaused: 0,
            ContainersStopped: 0);

    private static TaskCompletionSource NewCompletionSource()
        => new(TaskCreationOptions.RunContinuationsAsynchronously);
}
