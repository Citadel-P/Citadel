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
            string.IsNullOrWhiteSpace(dto.ResourceType)
                ? EdgeAgentResourceType.Platform
                : Enum.Parse<EdgeAgentResourceType>(dto.ResourceType),
            dto.ResourceId ?? dto.PlatformId,
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
            string.IsNullOrWhiteSpace(dto.ResourceType)
                ? EdgeAgentResourceType.Platform
                : Enum.Parse<EdgeAgentResourceType>(dto.ResourceType),
            dto.ResourceId ?? dto.PlatformId,
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
            dto.UpdatedAtUtc,
            string.IsNullOrWhiteSpace(dto.Profile)
                ? EdgeAgentProfile.Ordinary
                : Enum.Parse<EdgeAgentProfile>(dto.Profile),
            dto.ClusterId,
            dto.DockerNodeId,
            dto.DockerDaemonId,
            dto.DockerHostname,
            dto.SwarmRole,
            dto.LastObservedServiceId,
            dto.LastObservedTaskId,
            dto.FirstEnrolledAtUtc,
            dto.LastAuthenticatedAtUtc,
            dto.RevocationReason);

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
                dto.PlatformDescription,
                ClusterId: dto.PlatformClusterId,
                PruneHistoricalSwarmTaskContainers: dto.PlatformPruneHistoricalSwarmTaskContainers).ToDomain(),
            dto.BindingId is null
                ? null
                : new EdgeAgentBindingDto(
                    dto.BindingId.Value,
                    dto.BindingPlatformId!.Value,
                    dto.BindingResourceType,
                    dto.BindingResourceId ?? dto.BindingPlatformId.Value,
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
                    dto.BindingUpdatedAtUtc!.Value,
                    dto.BindingProfile,
                    dto.BindingClusterId,
                    dto.BindingDockerNodeId,
                    dto.BindingDockerDaemonId,
                    dto.BindingDockerHostname,
                    dto.BindingSwarmRole,
                    dto.BindingLastObservedServiceId,
                    dto.BindingLastObservedTaskId,
                    dto.BindingFirstEnrolledAtUtc,
                    dto.BindingLastAuthenticatedAtUtc,
                    dto.BindingRevocationReason).ToDomain());

    public static SwarmNodeAgentInstallation ToDomain(this SwarmNodeAgentInstallationDto dto)
        => new(
            dto.PlatformId,
            dto.ClusterId,
            dto.ManagerDockerNodeId,
            dto.ManagerDockerDaemonId,
            dto.DockerServiceId,
            dto.DockerServiceName,
            dto.AgentImageReference,
            dto.AgentImageDigest,
            dto.DockerCaConfigId,
            dto.DockerCaConfigName,
            Enum.Parse<SwarmNodeAgentDesiredState>(dto.DesiredState),
            dto.OperationId,
            string.IsNullOrWhiteSpace(dto.OperationKind) ? null : Enum.Parse<SwarmNodeAgentOperationKind>(dto.OperationKind),
            string.IsNullOrWhiteSpace(dto.OperationState) ? null : Enum.Parse<SwarmNodeAgentOperationState>(dto.OperationState),
            dto.OperationStartedAtUtc,
            dto.OperationActorId,
            dto.OperationError,
            dto.CreatedAtUtc,
            dto.UpdatedAtUtc);

    public static SwarmNodeAgentBootstrap ToDomain(this SwarmNodeAgentBootstrapDto dto)
        => new(
            dto.Id,
            dto.PlatformId,
            dto.ClusterId,
            dto.Version,
            dto.TokenHash,
            dto.DockerSecretId,
            dto.DockerSecretName,
            dto.ExpiresAtUtc,
            dto.RevokedAtUtc,
            dto.CreatedByActorId,
            dto.CreatedAtUtc,
            dto.UpdatedAtUtc);
}
