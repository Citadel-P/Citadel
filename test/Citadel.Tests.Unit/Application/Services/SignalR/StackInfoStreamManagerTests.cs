using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class StackInfoStreamManagerTests
{
    [Fact]
    public async Task StackInfo_PreservesSwarmTaskIdentityAndStreamsNodeStats()
    {
        var stackId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var container = new Container(
            "/web.1.task-1",
            "image",
            platformId,
            "abcdef0123456789",
            ContainerStateStatus.Running,
            stackId: stackId,
            isSwarmTask: true,
            dockerNodeId: "worker-node");
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(repository => repository.GetContainersAsync(stackId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([container]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(work => work.Stacks).Returns(stacks.Object);
        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var initial = new TaskCompletionSource<DockerContainer>(TaskCreationOptions.RunContinuationsAsynchronously);
        var withStats = new TaskCompletionSource<DockerContainer>(TaskCreationOptions.RunContinuationsAsynchronously);
        var dispatcher = new Mock<IApplicationHubDispatcher>();
        dispatcher
            .Setup(value => value.SendStackContainersInfo(
                stackId,
                It.IsAny<IEnumerable<DockerContainer>>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, IEnumerable<DockerContainer>, CancellationToken>((_, containers, _) =>
            {
                var streamed = Assert.Single(containers);
                if (streamed.ContainerStat is null)
                    initial.TrySetResult(streamed);
                else
                    withStats.TrySetResult(streamed);
            })
            .Returns(Task.CompletedTask);
        var broadcaster = new ContainerStatsBroadcaster();
        var manager = new StackInfoStreamManager(
            services.GetRequiredService<IServiceScopeFactory>(),
            dispatcher.Object,
            broadcaster,
            Mock.Of<ILogger<StackInfoStreamManager>>());
        var groupId = Constants.WellKnownSignalRGroups.StackInfoGroup(stackId);

        manager.AddSubscriber(groupId, "connection-1");
        var initialContainer = await initial.Task.WaitAsync(
            TimeSpan.FromSeconds(2),
            TestContext.Current.CancellationToken);
        await broadcaster.PublishAsync(
            new ContainerStatsSnapshot(
                platformId,
                [new ContainerStat(container.Id, 128, 16, 7.5, 512, 10, 20, 123)]),
            TestContext.Current.CancellationToken);
        var updatedContainer = await withStats.Task.WaitAsync(
            TimeSpan.FromSeconds(2),
            TestContext.Current.CancellationToken);

        Assert.True(initialContainer.IsSwarmTask);
        Assert.Equal(stackId, initialContainer.StackId);
        Assert.True(updatedContainer.IsSwarmTask);
        Assert.Equal(7.5, updatedContainer.ContainerStat?.CpuUsage);
        Assert.Equal(128, updatedContainer.ContainerStat?.MemoryActive);
        manager.RemoveSubscriber(groupId, "connection-1");
    }
}
