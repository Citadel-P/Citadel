using Citadel.Edge.V1;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Google.Protobuf;
using Hosting.Common;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentSession(
    Domain.EdgeAgentResourceType resourceType,
    Guid resourceId,
    Guid platformId,
    Guid agentId,
    string agentFingerprint,
    string sessionId)
{
    private readonly ConcurrentDictionary<string, EdgePendingCommand> pendingCommands = new();
    private readonly Channel<CoreEnvelope> outbound =
        Channel.CreateBounded<CoreEnvelope>(Helpers.ChannelDefaultOptions(
            capacity: EdgeAgentDefaults.OutboundQueueCapacity,
            singleWriter: false));

    public Guid PlatformId { get; } = platformId;
    public Domain.EdgeAgentResourceType ResourceType { get; } = resourceType;
    public Guid ResourceId { get; } = resourceId;
    public EdgeAgentTarget Target { get; } = new(resourceType, resourceId);
    public Guid AgentId { get; } = agentId;
    public string AgentFingerprint { get; } = agentFingerprint;
    public string SessionId { get; } = sessionId;
    public ChannelReader<CoreEnvelope> Outbound => outbound.Reader;

    public async Task<EdgePendingCommand> SendCommandAsync(
        EdgeCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        bool expectsStream,
        CancellationToken cancellationToken)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            throw new InvalidOperationException("Edge Agent command payload exceeded the maximum payload size.");
        }

        if (pendingCommands.Count >= EdgeAgentDefaults.MaxConcurrentCommandsPerSession)
        {
            throw new InvalidOperationException("Edge Agent session has reached the maximum number of concurrent commands.");
        }

        if (expectsStream && pendingCommands.Values.Count(command => command.ExpectsStream) >= EdgeAgentDefaults.MaxActiveStreamsPerSession)
        {
            throw new InvalidOperationException("Edge Agent session has reached the maximum number of active streams.");
        }

        var commandId = Guid.CreateVersion7().ToString("D");
        var pending = new EdgePendingCommand(commandId, expectsStream);
        if (!pendingCommands.TryAdd(commandId, pending))
        {
            throw new InvalidOperationException("Failed to register Edge Agent command.");
        }

        var envelope = new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            SessionId = SessionId,
            CommandId = commandId,
            Command = new EdgeCommand
            {
                CommandId = commandId,
                PlatformId = PlatformId.ToString("D"),
                ResourceType = MapResourceType(ResourceType),
                ResourceId = ResourceId.ToString("D"),
                Kind = kind,
                Payload = ByteString.CopyFrom(payload),
                PayloadSchemaVersion = 1,
                TimeoutMs = (int)Math.Min(timeout.TotalMilliseconds, int.MaxValue),
                CorrelationId = correlationId ?? string.Empty,
                ExpectsStream = expectsStream
            }
        };

        try
        {
            await outbound.Writer.WriteAsync(envelope, cancellationToken);
            return pending;
        }
        catch
        {
            pendingCommands.TryRemove(commandId, out _);
            pending.Fail("Failed to send command to the Edge Agent session.");
            throw;
        }
    }

    public void RemovePending(string commandId, string? failureReason = "Edge Agent command was canceled.")
    {
        if (pendingCommands.TryRemove(commandId, out var pending))
        {
            if (failureReason is not null)
            {
                pending.Fail(failureReason);
            }
        }
    }

    public async Task SendStreamInputAsync(string commandId, byte[] payload, CancellationToken cancellationToken)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            throw new InvalidOperationException("Edge Agent stream input exceeded the maximum payload size.");
        }

        if (!pendingCommands.ContainsKey(commandId))
        {
            throw new InvalidOperationException("Edge Agent command is no longer pending.");
        }

        await outbound.Writer.WriteAsync(new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            SessionId = SessionId,
            CommandId = commandId,
            StreamInput = new StreamInput { Payload = ByteString.CopyFrom(payload) }
        }, cancellationToken);
    }

    public async Task CancelCommandAsync(string commandId, string reason, CancellationToken cancellationToken)
    {
        await outbound.Writer.WriteAsync(new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            SessionId = SessionId,
            CommandId = commandId,
            CancelCommand = new CancelCommand { Reason = reason }
        }, cancellationToken);

        RemovePending(commandId, reason);
    }

    public void HandleOutput(string commandId, byte[] payload)
    {
        if (pendingCommands.TryGetValue(commandId, out var pending) && !pending.Output(payload))
        {
            pendingCommands.TryRemove(commandId, out _);
        }
    }

    public void HandleCompleted(string commandId)
    {
        if (pendingCommands.TryRemove(commandId, out var pending))
        {
            pending.Complete();
        }
    }

    public void HandleFailed(string commandId, string message)
    {
        if (pendingCommands.TryRemove(commandId, out var pending))
        {
            pending.Fail(message);
        }
    }

    public void Enqueue(CoreEnvelope envelope)
        => outbound.Writer.TryWrite(envelope);

    public void Disconnect(string reason)
    {
        outbound.Writer.TryWrite(new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            SessionId = SessionId,
            Disconnect = new Disconnect { Reason = reason }
        });
        outbound.Writer.TryComplete();
        FailPending(reason);
    }

    public void Complete()
    {
        outbound.Writer.TryComplete();
        FailPending("Edge Agent session disconnected.");
    }

    private void FailPending(string reason)
    {
        foreach (var commandId in pendingCommands.Keys)
        {
            if (pendingCommands.TryRemove(commandId, out var pending))
            {
                pending.Fail(reason);
            }
        }
    }

    private static Citadel.Edge.V1.EdgeAgentResourceType MapResourceType(Domain.EdgeAgentResourceType resourceType)
        => resourceType == Domain.EdgeAgentResourceType.BuildAgentPool
            ? Citadel.Edge.V1.EdgeAgentResourceType.BuildAgentPool
            : Citadel.Edge.V1.EdgeAgentResourceType.Platform;
}
