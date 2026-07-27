using System.Collections.Immutable;
using System.Runtime.CompilerServices;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Hosting.Common;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.Services.SignalR;

public sealed class ExecSessionManagerTests
{
    [Fact]
    public async Task StartExecProcess_RequiresAnExistingSubscription()
    {
        var cache = new Mock<IPlatformContainerCache>(MockBehavior.Strict);
        var manager = CreateManager(cache: cache.Object);

        await manager.StartExecProcess(
            "0123456789abcdef",
            "session-1",
            "sh",
            TestContext.Current.CancellationToken);

        cache.VerifyNoOtherCalls();
    }

    [Fact]
    public async Task RemovingLastSubscriber_CancelsAndDisposesExecSession()
    {
        const string containerId = "0123456789abcdef";
        const string normalizedContainerId = "0123456789ab";
        const string sessionId = "session-1";
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, sessionId);

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

        var cacheEntry = new PlatformCacheEntry(
            Guid.CreateVersion7(),
            "https://platform.example",
            PlatformConnectorType.Agent,
            ImmutableDictionary<string, Guid>.Empty);
        var cache = new Mock<IPlatformContainerCache>();
        cache
            .Setup(x => x.TryGetPlatformWithContainer(normalizedContainerId, out cacheEntry))
            .Returns(true);

        var manager = CreateManager(connectorFactory.Object, cache.Object);
        manager.AddSubscriber(groupId, "connection-1");

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

        manager.RemoveSubscriber(groupId, "connection-1");

        await sessionDisposed.Task.WaitAsync(
            TimeSpan.FromSeconds(2),
            TestContext.Current.CancellationToken);
        session.Verify(x => x.DisposeAsync(), Times.Once);
    }

    private static ExecSessionManager CreateManager(
        IConnectorFactory<IContainerConnector>? connectorFactory = null,
        IPlatformContainerCache? cache = null)
        => new(
            Mock.Of<IApplicationHubDispatcher>(),
            Mock.Of<ILogger<ExecSessionManager>>(),
            connectorFactory ?? Mock.Of<IConnectorFactory<IContainerConnector>>(),
            cache ?? Mock.Of<IPlatformContainerCache>());

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamUntilCancelled(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
        yield break;
    }
}
