using System.Runtime.CompilerServices;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class ExecSessionManagerTests
{
    [Fact]
    public async Task StartExecProcess_RequiresAnExistingSubscription()
    {
        var manager = CreateManager();

        await manager.StartExecProcess(
            "0123456789abcdef",
            "session-1",
            "sh",
            TestContext.Current.CancellationToken);

    }

    [Fact]
    public async Task RemovingLastSubscriber_CancelsAndDisposesExecSession()
    {
        const string containerId = "0123456789abcdef";
        const string normalizedContainerId = "0123456789ab";
        const string sessionId = "session-1";
        var sessionDisposed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var session = new Mock<IExecSession>();
        session.SetupGet(x => x.Output).Returns(StreamUntilCancelled());
        session
            .Setup(x => x.DisposeAsync())
            .Returns(() =>
            {
                sessionDisposed.TrySetResult();
                return ValueTask.CompletedTask;
            });

        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(x => x.ExecAsync(
                "https://platform.example",
                normalizedContainerId,
                "/bin/sh",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(session.Object);

        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        connectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);

        var platform = new Platform(
            "platform",
            "https://platform.example",
            0,
            0,
            0,
            1,
            1024,
            "1.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Agent,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
        var container = new Container(
            "container",
            "image",
            platform.Id,
            normalizedContainerId,
            ContainerStateStatus.Running);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByIdAsync(normalizedContainerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(container);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Containers).Returns(containers.Object);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();

        var manager = CreateManager(
            connectorFactory.Object,
            services.GetRequiredService<IServiceScopeFactory>());
        manager.AddSubscriber(
            Constants.WellKnownSignalRGroups.ContainerExecGroup(normalizedContainerId, sessionId),
            "connection-1");

        await manager.StartExecProcess(
            containerId,
            sessionId,
            "sh",
            TestContext.Current.CancellationToken);
        connector.Verify(
            x => x.ExecAsync(
                "https://platform.example",
                normalizedContainerId,
                "/bin/sh",
                It.IsAny<CancellationToken>()),
            Times.Once);

        manager.RemoveSubscriber(
            Constants.WellKnownSignalRGroups.ContainerExecGroup(normalizedContainerId, sessionId),
            "connection-1");

        await sessionDisposed.Task.WaitAsync(
            TimeSpan.FromSeconds(2),
            TestContext.Current.CancellationToken);
        session.Verify(x => x.DisposeAsync(), Times.Once);
    }

    [Fact]
    public async Task StartExecProcess_ResourceId_RoutesToPersistedOwningNode()
    {
        const string dockerContainerId = "fedcba9876543210";
        const string dockerNodeId = "worker-node";
        const string sessionId = "session-node";
        var platform = new Platform(
            "swarm",
            "https://manager.example",
            0,
            0,
            0,
            1,
            1024,
            "1.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.EdgeAgent,
            new DockerPlatformDescriptor("manager", 0, 0, 0, 0));
        var container = new Container(
            "task",
            "image",
            platform.Id,
            dockerContainerId,
            ContainerStateStatus.Running,
            dockerNodeId: dockerNodeId);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByIdAsync(container.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(container);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Containers).Returns(containers.Object);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        var services = new ServiceCollection().AddSingleton(unitOfWork.Object).BuildServiceProvider();

        var routed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var session = new Mock<IExecSession>();
        session.SetupGet(x => x.Output).Returns(StreamUntilCancelled());
        session.Setup(x => x.DisposeAsync()).Returns(ValueTask.CompletedTask);
        var runtime = new Mock<ISwarmNodeRuntimeConnector>();
        runtime
            .Setup(x => x.ExecAsync(
                platform,
                dockerNodeId,
                dockerContainerId,
                "/bin/sh",
                It.IsAny<CancellationToken>()))
            .Callback(routed.SetResult)
            .ReturnsAsync(session.Object);
        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>(MockBehavior.Strict);
        var manager = CreateManager(
            connectorFactory.Object,
            services.GetRequiredService<IServiceScopeFactory>(),
            runtime.Object);
        var resourceReference = container.Id.ToString("D");
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(resourceReference, sessionId);
        manager.AddSubscriber(groupId, "connection-1");

        await manager.StartExecProcess(
            resourceReference,
            sessionId,
            "sh",
            TestContext.Current.CancellationToken);
        await routed.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);

        runtime.VerifyAll();
        connectorFactory.VerifyNoOtherCalls();
        manager.RemoveSubscriber(groupId, "connection-1");
    }

    private static ExecSessionManager CreateManager(
        IConnectorFactory<IContainerConnector>? connectorFactory = null,
        IServiceScopeFactory? scopeFactory = null,
        ISwarmNodeRuntimeConnector? swarmNodeRuntimeConnector = null)
        => new(
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<ExecSessionManager>>(),
            connectorFactory ?? Mock.Of<IConnectorFactory<IContainerConnector>>(),
            scopeFactory ?? new ServiceCollection()
                .AddSingleton(Mock.Of<IUnitOfWork>())
                .BuildServiceProvider()
                .GetRequiredService<IServiceScopeFactory>(),
            swarmNodeRuntimeConnector ?? Mock.Of<ISwarmNodeRuntimeConnector>());

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamUntilCancelled(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
        yield break;
    }
}
