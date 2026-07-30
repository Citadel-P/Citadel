using Domain.Entities.Platforms;

namespace Domain.Contracts.Resources.Platforms;

public sealed record EdgeAgentEnrollmentResult(
    Guid EnrollmentId,
    Guid PlatformId,
    string Token,
    DateTime ExpiresAtUtc,
    EdgeAgentEnrollmentInstructions Instructions,
    EdgeAgentResourceType ResourceType = EdgeAgentResourceType.Platform,
    Guid? ResourceId = null);

public sealed record EdgeAgentEnrollmentInstructions(
    string CoreUrl,
    IReadOnlyDictionary<string, string> Environment,
    string AgentImage,
    string DockerRunCommand);

public sealed record EdgeAgentStatusResult(
    string ConnectionStatus,
    DateTime? LastConnectedAtUtc,
    DateTime? LastDisconnectedAtUtc,
    DateTime? LastHeartbeatAtUtc,
    string? LastSeenVersion,
    string? LastSeenHostname,
    string? AgentFingerprint,
    int? ProtocolVersion,
    DateTime? RevokedAtUtc,
    DateTime? EnrollmentExpiresAtUtc);

public sealed record EdgeAgentEnrollmentRequest(
    string EnrollmentToken,
    string AgentPublicKey,
    string AgentFingerprint,
    string Hostname,
    string AgentVersion,
    string CapabilitiesJson,
    int ProtocolVersion,
    string DaemonId);

public sealed record EdgeAgentEnrollmentCompleteResult(
    Guid PlatformId,
    Guid AgentId,
    EdgeAgentResourceType ResourceType = EdgeAgentResourceType.Platform,
    Guid? ResourceId = null);

public sealed record EdgeAgentHeartbeatSnapshot(
    bool DockerReachable,
    string? DockerVersion,
    string? Hostname,
    string? AgentVersion,
    string? CapabilitiesJson);

public sealed record EdgeAgentCommandRouterResult(
    byte[]? Payload,
    string? ErrorMessage)
{
    public bool IsSuccess => ErrorMessage is null;

    public static EdgeAgentCommandRouterResult Success(byte[] payload) => new(payload, null);
    public static EdgeAgentCommandRouterResult Failure(string message) => new(null, message);
}

public sealed record EdgeAgentInteractiveCommand(
    string CommandId,
    IAsyncEnumerable<EdgeAgentStreamItem> Output);

public sealed record EdgeAgentStreamItem(
    byte[]? Payload,
    string? ErrorMessage,
    bool Completed = false)
{
    public static EdgeAgentStreamItem Output(byte[] payload) => new(payload, null);
    public static EdgeAgentStreamItem Failure(string message) => new(null, message, true);
    public static EdgeAgentStreamItem Complete() => new(null, null, true);
}
