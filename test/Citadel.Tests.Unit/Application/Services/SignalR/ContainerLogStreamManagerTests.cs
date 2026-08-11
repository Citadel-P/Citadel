using Application.Services;
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

public sealed class ContainerLogStreamManagerTests
{
    [Fact]
    public async Task StartContainerLogs_FullDockerId_UsesNormalizedSubscriberGroup()
    {
        const string fullContainerId = "a4c05df3937c5d2479d48cbf6b15eb85e719c27c30205fc7c9b2f82b9b973750";
        const string normalizedContainerId = "a4c05df3937c";
        var queried = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByIdAsync(normalizedContainerId, It.IsAny<CancellationToken>()))
            .Callback(queried.SetResult)
            .Returns(Task.FromResult<global::Domain.Entities.Container?>(null));
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Containers).Returns(containers.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var manager = new ContainerLogStreamManager(
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<ContainerLogStreamManager>>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            new ContainerEventBroadcaster(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<ISwarmNodeRuntimeConnector>());
        var groupId = Constants.WellKnownSignalRGroups.ContainerLogGroup(normalizedContainerId);
        manager.AddSubscriber(groupId, "connection-1");

        manager.StartContainerLogs(fullContainerId);

        await queried.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        containers.Verify(
            x => x.GetByIdAsync(normalizedContainerId, It.IsAny<CancellationToken>()),
            Times.AtLeastOnce);
        manager.RemoveSubscriber(groupId, "connection-1");
    }

    [Fact]
    public async Task StartContainerLogs_ResourceId_RoutesToPersistedOwningNode()
    {
        const string dockerNodeId = "worker-node";
        const string dockerContainerId = "abcdef0123456789";
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
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var routed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var runtime = new Mock<ISwarmNodeRuntimeConnector>();
        runtime
            .Setup(x => x.StreamContainerLogsAsync(
                platform,
                dockerNodeId,
                dockerContainerId,
                It.IsAny<CancellationToken>()))
            .Callback(routed.SetResult)
            .Returns(EmptyLogStream());
        var manager = new ContainerLogStreamManager(
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<ContainerLogStreamManager>>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            new ContainerEventBroadcaster(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            runtime.Object);
        var resourceReference = container.Id.ToString("D");
        var groupId = Constants.WellKnownSignalRGroups.ContainerLogGroup(resourceReference);
        manager.AddSubscriber(groupId, "connection-1");

        manager.StartContainerLogs(resourceReference);
        await routed.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);

        runtime.VerifyAll();
        manager.RemoveSubscriber(groupId, "connection-1");
    }

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> EmptyLogStream()
    {
        await Task.CompletedTask;
        yield break;
    }
}
