namespace Infrastructure.Persistence.Dtos;

internal sealed record EdgeAgentEnrollmentDto(
    Guid Id,
    Guid PlatformId,
    string TokenHash,
    DateTime ExpiresAtUtc,
    DateTime? UsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid CreatedByActorId,
    DateTime CreatedAtUtc);

internal sealed record EdgeAgentBindingDto(
    Guid Id,
    Guid PlatformId,
    Guid AgentId,
    string AgentPublicKey,
    string AgentFingerprint,
    string ConnectionStatus,
    DateTime? LastConnectedAtUtc,
    DateTime? LastDisconnectedAtUtc,
    DateTime? LastHeartbeatAtUtc,
    string? LastSeenVersion,
    string? LastSeenHostname,
    string? CapabilitiesJson,
    int ProtocolVersion,
    DateTime? RevokedAtUtc,
    DateTime CreatedAtUtc,
    DateTime UpdatedAtUtc);
