namespace Domain.Entities.Platforms;

public sealed record EdgeAgentBinding(
    Guid Id,
    Guid PlatformId,
    Guid AgentId,
    string AgentPublicKey,
    string AgentFingerprint,
    EdgeAgentConnectionStatus ConnectionStatus,
    DateTime? LastConnectedAtUtc,
    DateTime? LastDisconnectedAtUtc,
    DateTime? LastHeartbeatAtUtc,
    string? LastSeenVersion,
    string? LastSeenHostname,
    string? CapabilitiesJson,
    int ProtocolVersion,
    DateTime? RevokedAtUtc,
    DateTime CreatedAtUtc,
    DateTime UpdatedAtUtc)
{
    public bool IsRevoked => RevokedAtUtc is not null || ConnectionStatus == EdgeAgentConnectionStatus.Revoked;
}
