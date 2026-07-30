using Citadel.Edge.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Google.Protobuf;
using Grpc.Core;
using LightResults;
using Microsoft.Extensions.Logging;
using NSec.Cryptography;
using System.Security.Cryptography;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentGrpcService(
    IEdgeAgentManagementService edgeAgentManagementService,
    EdgeAgentSessionRegistry sessionRegistry,
    ILogger<EdgeAgentGrpcService> logger)
    : EdgeAgentService.EdgeAgentServiceBase
{
    private const int NonceSize = 32;
    private const int SupportedProtocolVersion = EdgeAgentDefaults.ProtocolVersion;
    private static readonly string[] RequiredBuildAgentPoolCommands =
    [
        "images.build",
        "images.push",
        "images.checkBuildHost"
    ];
    private static readonly SignatureAlgorithm SignatureAlgorithm = SignatureAlgorithm.Ed25519;

    public override async Task Connect(
        IAsyncStreamReader<AgentEnvelope> requestStream,
        IServerStreamWriter<CoreEnvelope> responseStream,
        ServerCallContext context)
    {
        EdgeAgentSession? session = null;

        try
        {
            if (!await requestStream.MoveNext(context.CancellationToken))
            {
                return;
            }

            var authentication = await AuthenticateAsync(
                requestStream.Current,
                requestStream,
                responseStream,
                context.CancellationToken);

            if (authentication is null)
            {
                return;
            }

            session = sessionRegistry.Register(new EdgeAgentSession(
                authentication.ResourceType,
                authentication.ResourceId,
                authentication.PlatformId,
                authentication.AgentId,
                authentication.AgentFingerprint,
                Guid.CreateVersion7().ToString("D")));

            await edgeAgentManagementService.MarkConnectedAsync(
                authentication.ResourceType,
                authentication.ResourceId,
                authentication.Hostname,
                authentication.AgentVersion,
                authentication.CapabilitiesJson,
                DateTime.UtcNow,
                context.CancellationToken);

            logger.LogInformation(
                "Edge Agent session accepted for {ResourceType} {ResourceId} agent {AgentId}",
                authentication.ResourceType,
                authentication.ResourceId,
                authentication.AgentId);

            session.Enqueue(new CoreEnvelope
            {
                EnvelopeId = Guid.CreateVersion7().ToString("D"),
                SessionId = session.SessionId,
                SessionAccepted = new SessionAccepted
                {
                    PlatformId = session.PlatformId.ToString("D"),
                    AgentId = session.AgentId.ToString("D"),
                    SessionId = session.SessionId,
                    ResourceType = MapResourceType(session.ResourceType),
                    ResourceId = session.ResourceId.ToString("D")
                }
            });

            var writer = WriteLoopAsync(session, responseStream, context.CancellationToken);
            var reader = ReadLoopAsync(session, requestStream, context.CancellationToken);

            await Task.WhenAny(writer, reader);
            session.Complete();

            await SuppressCancellationAsync(reader);
            await SuppressCancellationAsync(writer);
        }
        catch (OperationCanceledException) when (context.CancellationToken.IsCancellationRequested)
        {
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Edge Agent session failed");
        }
        finally
        {
            if (session is not null)
            {
                var shouldMarkDisconnected =
                    sessionRegistry.RemoveAndShouldMarkDisconnected(session);
                session.Complete();
                if (shouldMarkDisconnected)
                {
                    await edgeAgentManagementService.MarkDisconnectedAsync(
                        session.ResourceType,
                        session.ResourceId,
                        DateTime.UtcNow,
                        CancellationToken.None);
                    logger.LogInformation("Edge Agent session disconnected for {ResourceType} {ResourceId}", session.ResourceType, session.ResourceId);
                }
            }
        }
    }

    private async Task<EdgeAgentAuthentication?> AuthenticateAsync(
        AgentEnvelope firstEnvelope,
        IAsyncStreamReader<AgentEnvelope> requestStream,
        IServerStreamWriter<CoreEnvelope> responseStream,
        CancellationToken cancellationToken)
    {
        if (firstEnvelope.EnrollmentRequest is { } enrollmentRequest)
        {
            return await CompleteEnrollmentAsync(enrollmentRequest, responseStream, cancellationToken);
        }

        if (firstEnvelope.Hello is { } hello)
        {
            return await AuthenticateReconnectAsync(hello, requestStream, responseStream, cancellationToken);
        }

        await RejectAsync(responseStream, "First Edge Agent message must be an enrollment request or hello.");
        return null;
    }

    private async Task<EdgeAgentAuthentication?> CompleteEnrollmentAsync(
        EnrollmentRequest request,
        IServerStreamWriter<CoreEnvelope> responseStream,
        CancellationToken cancellationToken)
    {
        if (request.PublicKey.Length == 0)
        {
            await RejectAsync(responseStream, "Enrollment request is missing the agent public key.");
            return null;
        }

        if (!TryValidateProtocolVersion(request.ProtocolVersion, out var protocolVersion, out var protocolError))
        {
            await RejectAsync(responseStream, protocolError);
            return null;
        }

        if (!EdgeAgentCapabilities.TryValidate(request.CapabilitiesJson, out var capabilitiesJson, out var capabilitiesError))
        {
            await RejectAsync(responseStream, capabilitiesError ?? "Edge Agent capabilities are invalid.");
            return null;
        }

        var publicKey = request.PublicKey.ToByteArray();
        var fingerprint = GetFingerprint(publicKey);
        var result = await edgeAgentManagementService.CompleteEnrollmentAsync(
            new EdgeAgentEnrollmentRequest(
                request.EnrollmentToken,
                Convert.ToBase64String(publicKey),
                fingerprint,
                request.Hostname,
                request.AgentVersion,
                capabilitiesJson,
                protocolVersion,
                request.DaemonId),
            DateTime.UtcNow,
            cancellationToken);

        if (!result.IsSuccess(out var enrollment, out var error))
        {
            await RejectAsync(responseStream, error?.Message ?? "Enrollment failed.");
            return null;
        }

        return new EdgeAgentAuthentication(
            enrollment.PlatformId,
            enrollment.ResourceType,
            enrollment.ResourceId ?? enrollment.PlatformId,
            enrollment.AgentId,
            fingerprint,
            request.Hostname,
            request.AgentVersion,
            capabilitiesJson);
    }

    private async Task<EdgeAgentAuthentication?> AuthenticateReconnectAsync(
        AgentHello hello,
        IAsyncStreamReader<AgentEnvelope> requestStream,
        IServerStreamWriter<CoreEnvelope> responseStream,
        CancellationToken cancellationToken)
    {
        var resourceType = MapResourceType(hello.ResourceType);
        var resourceIdText = string.IsNullOrWhiteSpace(hello.ResourceId) ? hello.PlatformId : hello.ResourceId;
        if (!Guid.TryParse(hello.PlatformId, out var platformId) ||
            !Guid.TryParse(resourceIdText, out var resourceId) ||
            !Guid.TryParse(hello.AgentId, out var agentId))
        {
            await RejectAsync(responseStream, "Edge Agent hello contains an invalid target, platform, or agent id.");
            return null;
        }

        if (!TryValidateProtocolVersion(hello.ProtocolVersion, out _, out var protocolError))
        {
            await RejectAsync(responseStream, protocolError);
            return null;
        }

        if (!EdgeAgentCapabilities.TryValidate(hello.CapabilitiesJson, out var capabilitiesJson, out var capabilitiesError))
        {
            await RejectAsync(responseStream, capabilitiesError ?? "Edge Agent capabilities are invalid.");
            return null;
        }

        if (resourceType == Domain.EdgeAgentResourceType.BuildAgentPool &&
            !EdgeAgentCapabilities.HasRequiredCommands(capabilitiesJson, RequiredBuildAgentPoolCommands, out var missingCommand))
        {
            await RejectAsync(responseStream, $"Build pool Edge Agent must advertise capability '{missingCommand}'.");
            return null;
        }

        var bindingResult = await edgeAgentManagementService.GetReconnectBindingAsync(
            resourceType,
            resourceId,
            agentId,
            hello.AgentFingerprint,
            hello.DaemonId,
            cancellationToken);

        if (!bindingResult.IsSuccess(out var binding, out var error))
        {
            await RejectAsync(responseStream, error?.Message ?? "Edge Agent authentication failed.");
            return null;
        }

        var nonce = RandomNumberGenerator.GetBytes(NonceSize);
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        await responseStream.WriteAsync(new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            AuthChallenge = new AuthChallenge
            {
                Nonce = ByteString.CopyFrom(nonce),
                TimestampUnixSeconds = timestamp
            }
        });

        if (!await requestStream.MoveNext(cancellationToken) ||
            requestStream.Current.AuthChallengeResponse is not { } challengeResponse)
        {
            await RejectAsync(responseStream, "Edge Agent did not answer the authentication challenge.");
            return null;
        }

        if (!nonce.SequenceEqual(challengeResponse.Nonce.ToByteArray()) ||
            challengeResponse.TimestampUnixSeconds != timestamp)
        {
            await RejectAsync(responseStream, "Edge Agent authentication challenge response did not match.");
            return null;
        }

        var publicKeyBytes = Convert.FromBase64String(binding.AgentPublicKey);
        var publicKey = PublicKey.Import(SignatureAlgorithm, publicKeyBytes, KeyBlobFormat.RawPublicKey);
        var signedPayload = EdgeAgentChallengeSigning.BuildPayload(timestamp, nonce);
        var signature = challengeResponse.Signature.ToByteArray();
        var verified = SignatureAlgorithm.Verify(publicKey, signedPayload, signature);
        CryptographicOperations.ZeroMemory(signedPayload);

        if (!verified)
        {
            await RejectAsync(responseStream, "Edge Agent authentication signature is invalid.");
            return null;
        }

        return new EdgeAgentAuthentication(
            platformId,
            binding.NormalizedResourceType,
            binding.NormalizedResourceId,
            agentId,
            hello.AgentFingerprint,
            hello.Hostname,
            hello.AgentVersion,
            capabilitiesJson);
    }

    private async Task ReadLoopAsync(
        EdgeAgentSession session,
        IAsyncStreamReader<AgentEnvelope> requestStream,
        CancellationToken cancellationToken)
    {
        while (await requestStream.MoveNext(cancellationToken))
        {
            var envelope = requestStream.Current;
            if (envelope.Heartbeat is { } heartbeat)
            {
                var capabilitiesJson = EdgeAgentCapabilities.NormalizeHeartbeatCapabilities(heartbeat.CapabilitiesJson);
                if (capabilitiesJson is null)
                {
                    logger.LogWarning("Ignoring invalid Edge Agent heartbeat capabilities for {ResourceType} {ResourceId}", session.ResourceType, session.ResourceId);
                }

                await edgeAgentManagementService.MarkHeartbeatAsync(
                    session.ResourceType,
                    session.ResourceId,
                    new EdgeAgentHeartbeatSnapshot(
                        heartbeat.DockerReachable,
                        NullIfEmpty(heartbeat.DockerVersion),
                        NullIfEmpty(heartbeat.Hostname),
                        NullIfEmpty(heartbeat.AgentVersion),
                        capabilitiesJson),
                    DateTime.UtcNow,
                    cancellationToken);
                continue;
            }

            if (string.IsNullOrWhiteSpace(envelope.CommandId))
            {
                continue;
            }

            if (envelope.CommandOutput is { } output)
            {
                if (output.Payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
                {
                    session.HandleFailed(envelope.CommandId, "Edge Agent command output exceeded the maximum payload size.");
                    logger.LogWarning("Edge Agent command {CommandId} from platform {PlatformId} exceeded the maximum payload size", envelope.CommandId, session.PlatformId);
                    continue;
                }

                session.HandleOutput(envelope.CommandId, output.Payload.ToByteArray());
                continue;
            }

            if (envelope.CommandCompleted is not null)
            {
                session.HandleCompleted(envelope.CommandId);
                continue;
            }

            if (envelope.CommandFailed is { } failed)
            {
                session.HandleFailed(envelope.CommandId, string.IsNullOrWhiteSpace(failed.Message)
                    ? "Edge Agent command failed."
                    : failed.Message);
            }
        }
    }

    private static async Task WriteLoopAsync(
        EdgeAgentSession session,
        IServerStreamWriter<CoreEnvelope> responseStream,
        CancellationToken cancellationToken)
    {
        await foreach (var envelope in session.Outbound.ReadAllAsync(cancellationToken))
        {
            await responseStream.WriteAsync(envelope);
        }
    }

    private static async Task RejectAsync(IServerStreamWriter<CoreEnvelope> responseStream, string reason)
        => await responseStream.WriteAsync(new CoreEnvelope
        {
            EnvelopeId = Guid.CreateVersion7().ToString("D"),
            SessionRejected = new SessionRejected { Reason = reason }
        });

    private static async Task SuppressCancellationAsync(Task task)
    {
        try
        {
            await task;
        }
        catch (OperationCanceledException)
        {
        }
    }

    private static bool TryValidateProtocolVersion(int protocolVersion, out int normalizedProtocolVersion, out string error)
    {
        normalizedProtocolVersion = protocolVersion;
        error = string.Empty;

        if (protocolVersion == SupportedProtocolVersion)
        {
            return true;
        }

        error = $"Unsupported Edge Agent protocol version '{protocolVersion}'. Core supports version {SupportedProtocolVersion}.";
        return false;
    }

    private static string? NullIfEmpty(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value;

    private static string GetFingerprint(ReadOnlySpan<byte> publicKey)
    {
        var hash = SHA256.HashData(publicKey);
        return $"SHA256:{Convert.ToHexString(hash).ToLowerInvariant()}";
    }

    private sealed record EdgeAgentAuthentication(
        Guid PlatformId,
        Domain.EdgeAgentResourceType ResourceType,
        Guid ResourceId,
        Guid AgentId,
        string AgentFingerprint,
        string Hostname,
        string AgentVersion,
        string CapabilitiesJson);

    private static Domain.EdgeAgentResourceType MapResourceType(Citadel.Edge.V1.EdgeAgentResourceType resourceType)
        => resourceType == Citadel.Edge.V1.EdgeAgentResourceType.BuildAgentPool
            ? Domain.EdgeAgentResourceType.BuildAgentPool
            : Domain.EdgeAgentResourceType.Platform;

    private static Citadel.Edge.V1.EdgeAgentResourceType MapResourceType(Domain.EdgeAgentResourceType resourceType)
        => resourceType == Domain.EdgeAgentResourceType.BuildAgentPool
            ? Citadel.Edge.V1.EdgeAgentResourceType.BuildAgentPool
            : Citadel.Edge.V1.EdgeAgentResourceType.Platform;
}
