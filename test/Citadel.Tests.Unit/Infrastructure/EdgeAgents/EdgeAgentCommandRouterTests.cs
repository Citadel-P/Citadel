using Citadel.Edge.V1;
using Domain;
using Infrastructure.EdgeAgents;
using Microsoft.Extensions.Logging.Abstractions;

namespace Tests.Unit.Infrastructure.EdgeAgents;

public sealed class EdgeAgentCommandRouterTests
{
    [Fact]
    public async Task SendUnaryAsync_ShouldSendCancelCommand_WhenTimedOut()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var session = registry.Register(new EdgeAgentSession(
            platformId,
            Guid.CreateVersion7(),
            "SHA256:test",
            "session-1"));
        var router = new EdgeAgentCommandRouter(registry, NullLogger<EdgeAgentCommandRouter>.Instance);

        var task = router.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerList,
            [],
            TimeSpan.FromMilliseconds(10),
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

    private static async Task<CoreEnvelope> ReadOutboundAsync(EdgeAgentSession session, CancellationToken cancellationToken)
    {
        var readTask = session.Outbound.ReadAsync(cancellationToken).AsTask();
        var completed = await Task.WhenAny(readTask, Task.Delay(TimeSpan.FromSeconds(5), cancellationToken));
        Assert.Same(readTask, completed);
        return await readTask;
    }
}
