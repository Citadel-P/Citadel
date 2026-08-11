namespace Domain.Entities.Platforms;

public sealed record SwarmNodeAgentInstallation(
    Guid PlatformId,
    string ClusterId,
    string ManagerDockerNodeId,
    string ManagerDockerDaemonId,
    string? DockerServiceId,
    string DockerServiceName,
    string AgentImageReference,
    string AgentImageDigest,
    string? DockerCaConfigId,
    string? DockerCaConfigName,
    SwarmNodeAgentDesiredState DesiredState,
    Guid? OperationId,
    SwarmNodeAgentOperationKind? OperationKind,
    SwarmNodeAgentOperationState? OperationState,
    DateTime? OperationStartedAtUtc,
    Guid? OperationActorId,
    string? OperationError,
    DateTime CreatedAtUtc,
    DateTime UpdatedAtUtc);

public sealed record SwarmNodeAgentBootstrap(
    Guid Id,
    Guid PlatformId,
    string ClusterId,
    int Version,
    string TokenHash,
    string? DockerSecretId,
    string DockerSecretName,
    DateTime ExpiresAtUtc,
    DateTime? RevokedAtUtc,
    Guid CreatedByActorId,
    DateTime CreatedAtUtc,
    DateTime UpdatedAtUtc)
{
    public bool IsActive(DateTime utcNow) => RevokedAtUtc is null && ExpiresAtUtc > utcNow;
}
