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

    public static EdgeAgentPlatformState ToDomain(this EdgeAgentPlatformStateDto dto)
        => new(
            new PlatformDto(
                dto.PlatformId,
                dto.PlatformName,
                dto.PlatformAddress,
                dto.PlatformNetworkCount,
                dto.PlatformVolumeCount,
                dto.PlatformImageCount,
                dto.PlatformCpuCount,
                dto.PlatformMemTotal,
                dto.PlatformStatus,
                dto.PlatformConnectorType,
                dto.PlatformDescriptor,
                dto.PlatformServerVersion,
                dto.PlatformAgentVersion,
                dto.PlatformDescription).ToDomain(),
            dto.BindingId is null
                ? null
                : new EdgeAgentBindingDto(
                    dto.BindingId.Value,
                    dto.BindingPlatformId!.Value,
                    dto.BindingAgentId!.Value,
                    dto.BindingAgentPublicKey!,
                    dto.BindingAgentFingerprint!,
                    dto.BindingConnectionStatus!,
                    dto.BindingLastConnectedAtUtc,
                    dto.BindingLastDisconnectedAtUtc,
                    dto.BindingLastHeartbeatAtUtc,
                    dto.BindingLastSeenVersion,
                    dto.BindingLastSeenHostname,
                    dto.BindingCapabilitiesJson,
                    dto.BindingProtocolVersion!.Value,
                    dto.BindingRevokedAtUtc,
                    dto.BindingCreatedAtUtc!.Value,
                    dto.BindingUpdatedAtUtc!.Value).ToDomain());
}
