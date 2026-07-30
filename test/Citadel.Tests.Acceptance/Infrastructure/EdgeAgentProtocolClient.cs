using System.Buffers.Binary;
using System.Security.Cryptography;
using Citadel.Edge.V1;
using Citadel.Platforms.V1;
using Google.Protobuf;
using Grpc.Core;
using Grpc.Net.Client;
using NSec.Cryptography;

namespace Tests.Acceptance.Infrastructure;

internal sealed class EdgeAgentProtocolClient : IAsyncDisposable
{
    private const int ProtocolVersion = 2;
    private const string DaemonId = "acceptance-edge-daemon";
    private const string CapabilitiesJson =
        """{"commands":["platform.checkHealth","platform.prune","containers.list","containers.logs"]}""";
    private static readonly SignatureAlgorithm SignatureAlgorithm =
        SignatureAlgorithm.Ed25519;
    private static readonly Marshaller<AgentEnvelope> AgentMarshaller =
        Marshallers.Create(
            envelope => envelope.ToByteArray(),
            AgentEnvelope.Parser.ParseFrom);
    private static readonly Marshaller<CoreEnvelope> CoreMarshaller =
        Marshallers.Create(
            envelope => envelope.ToByteArray(),
            CoreEnvelope.Parser.ParseFrom);
    private static readonly Method<AgentEnvelope, CoreEnvelope> ConnectMethod =
        new(
            MethodType.DuplexStreaming,
            "citadel.edge.v1.EdgeAgentService",
            "Connect",
            AgentMarshaller,
            CoreMarshaller);

    private readonly Key privateKey = new(
        SignatureAlgorithm,
        new KeyCreationParameters
        {
            ExportPolicy = KeyExportPolicies.AllowPlaintextExport
        });
    private readonly SemaphoreSlim requestWriterLock = new(1, 1);
    private GrpcChannel? channel;
    private AsyncDuplexStreamingCall<AgentEnvelope, CoreEnvelope>? call;
    private Task? responseTask;
    private TaskCompletionSource connectionFinished = NewCompletionSource();

    public Guid PlatformId { get; private set; }

    public Guid AgentId { get; private set; }

    public string SessionId { get; private set; } = string.Empty;

    public int CompletedCommandCount { get; private set; }

    private byte[] PublicKey =>
        privateKey.PublicKey.Export(KeyBlobFormat.RawPublicKey);

    private string Fingerprint
    {
        get
        {
            var hash = SHA256.HashData(PublicKey);
            return $"SHA256:{Convert.ToHexString(hash).ToLowerInvariant()}";
        }
    }

    public async Task EnrollAsync(
        Uri grpcAddress,
        string enrollmentToken,
        CancellationToken cancellationToken)
    {
        await OpenConnectionAsync(grpcAddress, cancellationToken);
        await WriteAsync(
            new AgentEnvelope
            {
                EnvelopeId = Guid.CreateVersion7().ToString("D"),
                EnrollmentRequest = new EnrollmentRequest
                {
                    EnrollmentToken = enrollmentToken,
                    PublicKey = ByteString.CopyFrom(PublicKey),
                    Hostname = "acceptance-edge-agent",
                    AgentVersion = "acceptance",
                    ProtocolVersion = ProtocolVersion,
                    CapabilitiesJson = CapabilitiesJson,
                    DaemonId = DaemonId
                }
            },
            cancellationToken);

        var rejection = await CompleteHandshakeAsync(
            answerChallenge: false,
            cancellationToken);
        Assert.Null(rejection);
    }

    public async Task<string?> ReconnectAsync(
        Uri grpcAddress,
        CancellationToken cancellationToken)
    {
        Assert.NotEqual(Guid.Empty, PlatformId);
        Assert.NotEqual(Guid.Empty, AgentId);

        await OpenConnectionAsync(grpcAddress, cancellationToken);
        await WriteAsync(
            new AgentEnvelope
            {
                EnvelopeId = Guid.CreateVersion7().ToString("D"),
                Hello = new AgentHello
                {
                    PlatformId = PlatformId.ToString("D"),
                    ResourceType = EdgeAgentResourceType.Platform,
                    ResourceId = PlatformId.ToString("D"),
                    AgentId = AgentId.ToString("D"),
                    AgentFingerprint = Fingerprint,
                    Hostname = "acceptance-edge-agent",
                    AgentVersion = "acceptance",
                    ProtocolVersion = ProtocolVersion,
                    CapabilitiesJson = CapabilitiesJson,
                    DaemonId = DaemonId
                }
            },
            cancellationToken);

        return await CompleteHandshakeAsync(
            answerChallenge: true,
            cancellationToken);
    }

    public async Task SendHeartbeatAsync(CancellationToken cancellationToken)
    {
        Assert.False(string.IsNullOrWhiteSpace(SessionId));
        await WriteAsync(
            new AgentEnvelope
            {
                EnvelopeId = Guid.CreateVersion7().ToString("D"),
                SessionId = SessionId,
                Heartbeat = new AgentHeartbeat
                {
                    DockerReachable = true,
                    DockerVersion = "acceptance",
                    Hostname = "acceptance-edge-agent",
                    AgentVersion = "acceptance",
                    CapabilitiesJson = CapabilitiesJson
                }
            },
            cancellationToken);
    }

    public async Task DisconnectAsync(CancellationToken cancellationToken)
    {
        var activeCall = call;
        if (activeCall is null)
        {
            return;
        }

        try
        {
            await activeCall.RequestStream.CompleteAsync();
            await connectionFinished.Task.WaitAsync(
                TimeSpan.FromSeconds(10),
                cancellationToken);
        }
        catch (RpcException)
        {
        }
        finally
        {
            await CloseConnectionAsync();
        }
    }

    public Task WaitForDisconnectAsync(
        TimeSpan timeout,
        CancellationToken cancellationToken)
        => connectionFinished.Task.WaitAsync(timeout, cancellationToken);

    public async ValueTask DisposeAsync()
    {
        await CloseConnectionAsync();
        requestWriterLock.Dispose();
        privateKey.Dispose();
    }

    private async Task OpenConnectionAsync(
        Uri grpcAddress,
        CancellationToken cancellationToken)
    {
        await CloseConnectionAsync();
        AppContext.SetSwitch(
            "System.Net.Http.SocketsHttpHandler.Http2UnencryptedSupport",
            true);
        channel = GrpcChannel.ForAddress(grpcAddress);
        call = channel.CreateCallInvoker().AsyncDuplexStreamingCall(
            ConnectMethod,
            host: null,
            new CallOptions(cancellationToken: cancellationToken));
        connectionFinished = NewCompletionSource();
        SessionId = string.Empty;
    }

    private async Task<string?> CompleteHandshakeAsync(
        bool answerChallenge,
        CancellationToken cancellationToken)
    {
        Assert.NotNull(call);

        while (await call.ResponseStream.MoveNext(cancellationToken))
        {
            var envelope = call.ResponseStream.Current;
            if (envelope.AuthChallenge is { } challenge)
            {
                Assert.True(
                    answerChallenge,
                    "Core unexpectedly challenged a new enrollment.");
                await WriteAsync(
                    BuildChallengeResponse(challenge),
                    cancellationToken);
                continue;
            }

            if (envelope.SessionRejected is { } rejected)
            {
                await CloseConnectionAsync();
                return rejected.Reason;
            }

            if (envelope.SessionAccepted is { } accepted)
            {
                Assert.False(
                    string.IsNullOrWhiteSpace(accepted.PlatformId) ||
                    string.IsNullOrWhiteSpace(accepted.AgentId),
                    $"Core returned an incomplete Edge Agent identity: {accepted}");
                var platformId = Guid.Parse(accepted.PlatformId);
                var agentId = Guid.Parse(accepted.AgentId);
                Assert.NotEqual(
                    Guid.Empty,
                    platformId);
                Assert.NotEqual(
                    Guid.Empty,
                    agentId);
                PlatformId = platformId;
                AgentId = agentId;
                SessionId = accepted.SessionId;
                responseTask = RunSessionAsync(call);
                return null;
            }
        }

        await CloseConnectionAsync();
        throw new InvalidOperationException(
            "Core closed the Edge Agent stream before accepting or rejecting it.");
    }

    private async Task RunSessionAsync(
        AsyncDuplexStreamingCall<AgentEnvelope, CoreEnvelope> activeCall)
    {
        try
        {
            while (await activeCall.ResponseStream.MoveNext())
            {
                var envelope = activeCall.ResponseStream.Current;
                if (envelope.Command is { } command)
                {
                    await HandleCommandAsync(command, CancellationToken.None);
                }

                if (envelope.Disconnect is not null)
                {
                    break;
                }
            }
        }
        catch (RpcException)
        {
        }
        catch (ObjectDisposedException)
        {
        }
        finally
        {
            connectionFinished.TrySetResult();
        }
    }

    private async Task HandleCommandAsync(
        EdgeCommand command,
        CancellationToken cancellationToken)
    {
        if (command.Kind != EdgeCommandKind.PlatformPrune)
        {
            await WriteAsync(
                new AgentEnvelope
                {
                    EnvelopeId = Guid.CreateVersion7().ToString("D"),
                    SessionId = SessionId,
                    CommandId = command.CommandId,
                    CommandFailed = new CommandFailed
                    {
                        Message =
                            $"Acceptance Edge Agent does not support '{command.Kind}'."
                    }
                },
                cancellationToken);
            return;
        }

        var request = PruneRequest.Parser.ParseFrom(command.Payload);
        var response = new PruneResponse
        {
            Resource = request.Resource,
            SpaceReclaimed = 4096
        };
        await WriteAsync(
            new AgentEnvelope
            {
                EnvelopeId = Guid.CreateVersion7().ToString("D"),
                SessionId = SessionId,
                CommandId = command.CommandId,
                CommandOutput = new CommandOutput
                {
                    Payload = ByteString.CopyFrom(response.ToByteArray())
                }
            },
            cancellationToken);
        CompletedCommandCount++;
    }

    private AgentEnvelope BuildChallengeResponse(AuthChallenge challenge)
    {
        var nonce = challenge.Nonce.ToByteArray();
        var payload = new byte[sizeof(long) + nonce.Length];
        BinaryPrimitives.WriteInt64LittleEndian(
            payload.AsSpan(0, sizeof(long)),
            challenge.TimestampUnixSeconds);
        nonce.CopyTo(payload.AsSpan(sizeof(long)));
        var signature = SignatureAlgorithm.Sign(privateKey, payload);
        CryptographicOperations.ZeroMemory(payload);

        return new AgentEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            AuthChallengeResponse = new AuthChallengeResponse
            {
                Nonce = ByteString.CopyFrom(nonce),
                TimestampUnixSeconds = challenge.TimestampUnixSeconds,
                Signature = ByteString.CopyFrom(signature)
            }
        };
    }

    private async Task WriteAsync(
        AgentEnvelope envelope,
        CancellationToken cancellationToken)
    {
        Assert.NotNull(call);
        await requestWriterLock.WaitAsync(cancellationToken);
        try
        {
            await call.RequestStream.WriteAsync(
                envelope,
                cancellationToken);
        }
        finally
        {
            requestWriterLock.Release();
        }
    }

    private async Task CloseConnectionAsync()
    {
        call?.Dispose();
        if (responseTask is not null)
        {
            try
            {
                await responseTask;
            }
            catch (OperationCanceledException)
            {
            }
        }

        channel?.Dispose();
        call = null;
        channel = null;
        responseTask = null;
        connectionFinished.TrySetResult();
    }

    private static TaskCompletionSource NewCompletionSource()
        => new(TaskCreationOptions.RunContinuationsAsynchronously);
}
