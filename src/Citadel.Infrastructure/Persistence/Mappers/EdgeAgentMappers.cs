using Domain;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class EdgeAgentMappers
{
    public static EdgeAgentEnrollment ToDomain(this EdgeAgentEnrollmentDto dto)
        => new(
            dto.Id,
            dto.PlatformId,
            dto.TokenHash,
            dto.ExpiresAtUtc,
            dto.UsedAtUtc,
            dto.RevokedAtUtc,
            dto.CreatedByActorId,
            dto.CreatedAtUtc);

    public static EdgeAgentBinding ToDomain(this EdgeAgentBindingDto dto)
        => new(
            dto.Id,
            dto.PlatformId,
            dto.AgentId,
            dto.AgentPublicKey,
            dto.AgentFingerprint,
            Enum.Parse<EdgeAgentConnectionStatus>(dto.ConnectionStatus),
            dto.LastConnectedAtUtc,
            dto.LastDisconnectedAtUtc,
            dto.LastHeartbeatAtUtc,
            dto.LastSeenVersion,
            dto.LastSeenHostname,
            dto.CapabilitiesJson,
            dto.ProtocolVersion,
            dto.RevokedAtUtc,
            dto.CreatedAtUtc,
            dto.UpdatedAtUtc);
}
