using System.Text;
using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class StackLogStreamManagerTests
{
    [Fact]
    public async Task StartStackLogs_RoutesSwarmTaskLogsThroughTheOwningNode()
    {
        var stackId = Guid.CreateVersion7();
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
            "/web.1.task",
            "image",
            platform.Id,
            dockerContainerId,
            ContainerStateStatus.Running,
            stackId: stackId,
            isSwarmTask: true,
            dockerNodeId: dockerNodeId);
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(repository => repository.GetContainersAsync(stackId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Stacks).Returns(stacks.Object);
        unitOfWork.SetupGet(work => work.Platforms).Returns(platforms.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var routed = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var runtime = new Mock<ISwarmNodeRuntimeConnector>();
        runtime
            .Setup(connector => connector.StreamContainerLogsAsync(
                platform,
                dockerNodeId,
                "abcdef012345",
                It.IsAny<CancellationToken>()))
            .Callback(routed.SetResult)
            .Returns(EmptyLogStream());
        var manager = new StackLogStreamManager(
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<StackLogStreamManager>>(),
            new ContainerEventBroadcaster(),
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            runtime.Object);
        var groupId = Constants.WellKnownSignalRGroups.StackLogGroup(stackId);
        manager.AddSubscriber(groupId, "connection-1");

        manager.StartStackLogs(stackId);

        await routed.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        runtime.VerifyAll();
        manager.RemoveSubscriber(groupId, "connection-1");
    }

    [Fact]
    public void PrefixContainerName_PreservesMessagesWithoutIntermediateStrings()
    {
        var input = Encoding.UTF8.GetBytes(
            "2026-07-27T12:00:00.000000000Z first line\r\nplain line\n");
        var prefix = "[api] "u8;

        using var result = StackLogStreamManager.PrefixContainerName(input, prefix);

        Assert.NotNull(result);
        Assert.Equal(
            "2026-07-27T12:00:00.000000000Z [api] first line\n[api] plain line",
            Encoding.UTF8.GetString(result.Span));
    }

    [Fact]
    public void PrefixContainerName_SuppressesTimestampOnlyLines()
    {
        var input = Encoding.UTF8.GetBytes(
            "2026-07-27T12:00:00.000000000Z\n2026-07-27T12:00:01.000000000Z  \r\n");

        using var result = StackLogStreamManager.PrefixContainerName(input, "[api] "u8);

        Assert.Null(result);
    }

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> EmptyLogStream()
    {
        await Task.CompletedTask;
        yield break;
    }
}
