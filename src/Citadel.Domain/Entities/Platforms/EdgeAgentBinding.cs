namespace Domain.Entities.Platforms;

public sealed record EdgeAgentBinding(
    Guid Id,
    Guid PlatformId,
    EdgeAgentResourceType ResourceType,
    Guid ResourceId,
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
    public EdgeAgentResourceType NormalizedResourceType => ResourceType;
    public Guid NormalizedResourceId => ResourceId == Guid.Empty ? PlatformId : ResourceId;
    public bool IsRevoked => RevokedAtUtc is not null || ConnectionStatus == EdgeAgentConnectionStatus.Revoked;
}

public sealed record EdgeAgentPlatformState(Platform Platform, EdgeAgentBinding? Binding);
