using Citadel.Edge.V1;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Infrastructure.EdgeAgents;
using Microsoft.Extensions.Logging.Abstractions;
using System.Diagnostics;

namespace Tests.Unit.Infrastructure.EdgeAgents;

public sealed class EdgeAgentCommandRouterTests
{
    [Fact]
    public async Task SendUnaryAsync_ShouldSendCancelCommand_WhenTimedOut()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var session = registry.Register(new EdgeAgentSession(
            global::Domain.EdgeAgentResourceType.Platform,
            platformId,
            platformId,
            Guid.CreateVersion7(),
            "SHA256:test",
            "session-1"));
        var router = new EdgeAgentCommandRouter(registry, NullLogger<EdgeAgentCommandRouter>.Instance);

        var task = router.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerList,
            [],
            TimeSpan.FromSeconds(1),
            correlationId: null,
            CancellationToken.None);

        var commandEnvelope = await ReadOutboundAsync(session, TestContext.Current.CancellationToken);
        var result = await task;
        var cancelEnvelope = await ReadOutboundAsync(session, TestContext.Current.CancellationToken);

        Assert.NotNull(commandEnvelope.Command);
        Assert.Equal(commandEnvelope.CommandId, cancelEnvelope.CommandId);
        Assert.NotNull(cancelEnvelope.CancelCommand);
        Assert.False(result.IsSuccess);
        Assert.Contains("timed out", result.ErrorMessage);
    }

    [Fact]
    public async Task SendUnaryAsync_ShouldRejectOversizedPayload()
    {
        var registry = new EdgeAgentSessionRegistry();
        var router = new EdgeAgentCommandRouter(registry, NullLogger<EdgeAgentCommandRouter>.Instance);

        var result = await router.SendUnaryAsync(
            Guid.CreateVersion7(),
            EdgeAgentCommandKind.ContainerList,
            new byte[EdgeAgentDefaults.MaxEnvelopePayloadBytes + 1],
            TimeSpan.FromSeconds(1),
            correlationId: null,
            CancellationToken.None);

        Assert.False(result.IsSuccess);
        Assert.Contains("maximum payload size", result.ErrorMessage);
    }

    [Fact]
    public async Task SendUnaryAsync_WithNodeId_ShouldRouteOnlyToOwningNode()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var first = CreateNodeSession(platformId, "node-1");
        var second = CreateNodeSession(platformId, "node-2");
        registry.Register(first);
        registry.Register(second);
        var router = new EdgeAgentCommandRouter(registry, NullLogger<EdgeAgentCommandRouter>.Instance);

        var resultTask = router.SendUnaryAsync(
            platformId,
            "node-2",
            EdgeAgentCommandKind.ContainerInspect,
            [],
            TimeSpan.FromSeconds(5),
            correlationId: null,
            TestContext.Current.CancellationToken);

        var envelope = await ReadOutboundAsync(second, TestContext.Current.CancellationToken);
        Assert.False(first.Outbound.TryRead(out _));
        Assert.Equal("node-2", envelope.Command.NodeId);
        second.HandleCompleted(envelope.CommandId);

        var result = await resultTask;
        Assert.True(result.IsSuccess);
    }

    [Fact]
    public async Task SendUnaryAsync_WithNodeId_ShouldRejectManagerOnlyAndUndeliveredCommandsBeforeRouting()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var session = CreateNodeSession(platformId, "node-1");
        registry.Register(session);
        var router = new EdgeAgentCommandRouter(registry, NullLogger<EdgeAgentCommandRouter>.Instance);

        var deniedKinds = new[]
        {
            EdgeAgentCommandKind.SwarmNodeUpdate,
            EdgeAgentCommandKind.ContainerCreate,
            EdgeAgentCommandKind.ContainerExecBinary,
            EdgeAgentCommandKind.ImageList,
            EdgeAgentCommandKind.VolumeList,
            EdgeAgentCommandKind.NetworkList
        };
        foreach (var kind in deniedKinds)
        {
            var result = await router.SendUnaryAsync(
                platformId,
                "node-1",
                kind,
                [],
                TimeSpan.FromSeconds(1),
                correlationId: null,
                TestContext.Current.CancellationToken);

            Assert.False(result.IsSuccess);
            Assert.Contains("not allowed", result.ErrorMessage);
        }
        Assert.False(session.Outbound.TryRead(out _));
    }

    [Fact]
    public async Task CancelCommandAsync_ShouldReleasePendingCommand_WhenOutboundQueueIsFull()
    {
        var platformId = Guid.CreateVersion7();
        var session = CreateSession(platformId);
        var pending = await session.SendCommandAsync(
            Citadel.Edge.V1.EdgeCommandKind.ContainerList,
            [],
            TimeSpan.FromMinutes(1),
            correlationId: null,
            expectsStream: false,
            TestContext.Current.CancellationToken);

        for (var i = 1; i < EdgeAgentDefaults.OutboundQueueCapacity; i++)
            session.Enqueue(new CoreEnvelope { EnvelopeId = Guid.CreateVersion7().ToString("D") });

        var stopwatch = Stopwatch.StartNew();
        await session.CancelCommandAsync(
            pending.CommandId,
            "test cancellation",
            CancellationToken.None);

        Assert.True(stopwatch.Elapsed < TimeSpan.FromSeconds(1));
        Assert.Equal("test cancellation", pending.FailureReason);

        while (session.Outbound.TryRead(out _))
        {
        }

        await session.Outbound.Completion.WaitAsync(
            TimeSpan.FromSeconds(1),
            TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task CancelCommandAsync_WithCancelledToken_ShouldKeepCommandPending()
    {
        var platformId = Guid.CreateVersion7();
        var session = CreateSession(platformId);
        var pending = await session.SendCommandAsync(
            Citadel.Edge.V1.EdgeCommandKind.ContainerList,
            [],
            TimeSpan.FromMinutes(1),
            correlationId: null,
            expectsStream: false,
            TestContext.Current.CancellationToken);
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            session.CancelCommandAsync(pending.CommandId, "test cancellation", cancellation.Token));

        Assert.Null(pending.FailureReason);
        session.HandleFailed(pending.CommandId, "remote failure");
        Assert.Equal("remote failure", pending.FailureReason);
    }

    [Fact]
    public async Task PendingOutputOverflow_ShouldRetainExplicitFailure()
    {
        var platformId = Guid.CreateVersion7();
        var session = CreateSession(platformId);
        var pending = await session.SendCommandAsync(
            Citadel.Edge.V1.EdgeCommandKind.ContainerList,
            [],
            TimeSpan.FromMinutes(1),
            correlationId: null,
            expectsStream: true,
            TestContext.Current.CancellationToken);

        for (var i = 0; i <= EdgeAgentDefaults.PendingOutputQueueCapacity; i++)
            session.HandleOutput(pending.CommandId, [(byte)i]);

        var outputs = new List<EdgeAgentStreamItem>();
        await foreach (var item in pending.Reader.ReadAllAsync(TestContext.Current.CancellationToken))
            outputs.Add(item);

        Assert.Equal(EdgeAgentDefaults.PendingOutputQueueCapacity, outputs.Count);
        Assert.Equal("Edge Agent command output queue is full.", pending.FailureReason);
    }

    [Fact]
    public async Task SendCommandAsync_ShouldEnforceAtomicCommandLimitAndReleaseCapacity()
    {
        var platformId = Guid.CreateVersion7();
        var session = CreateSession(platformId);
        var pending = new List<EdgePendingCommand>();

        for (var i = 0; i < EdgeAgentDefaults.MaxConcurrentCommandsPerSession; i++)
        {
            pending.Add(await session.SendCommandAsync(
                Citadel.Edge.V1.EdgeCommandKind.ContainerList,
                [],
                TimeSpan.FromMinutes(1),
                correlationId: null,
                expectsStream: false,
                TestContext.Current.CancellationToken));
        }

        await Assert.ThrowsAsync<InvalidOperationException>(() =>
            session.SendCommandAsync(
                Citadel.Edge.V1.EdgeCommandKind.ContainerList,
                [],
                TimeSpan.FromMinutes(1),
                correlationId: null,
                expectsStream: false,
                TestContext.Current.CancellationToken));

        session.RemovePending(pending[0].CommandId, failureReason: null);
        var replacement = await session.SendCommandAsync(
            Citadel.Edge.V1.EdgeCommandKind.ContainerList,
            [],
            TimeSpan.FromMinutes(1),
            correlationId: null,
            expectsStream: false,
            TestContext.Current.CancellationToken);

        Assert.NotNull(replacement);
    }

    private static async Task<CoreEnvelope> ReadOutboundAsync(EdgeAgentSession session, CancellationToken cancellationToken)
    {
        var readTask = session.Outbound.ReadAsync(cancellationToken).AsTask();
        var completed = await Task.WhenAny(readTask, Task.Delay(TimeSpan.FromSeconds(5), cancellationToken));
        Assert.Same(readTask, completed);
        return await readTask;
    }

    private static EdgeAgentSession CreateSession(Guid platformId)
        => new(
            global::Domain.EdgeAgentResourceType.Platform,
            platformId,
            platformId,
            Guid.CreateVersion7(),
            "SHA256:test",
            Guid.CreateVersion7().ToString("D"));

    private static EdgeAgentSession CreateNodeSession(Guid platformId, string dockerNodeId)
        => new(
            global::Domain.EdgeAgentResourceType.Platform,
            platformId,
            platformId,
            Guid.CreateVersion7(),
            "SHA256:test",
            Guid.CreateVersion7().ToString("D"),
            global::Domain.EdgeAgentProfile.SwarmNode,
            dockerNodeId);
}
