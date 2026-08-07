namespace Infrastructure.Persistence.Dtos;

internal sealed record SwarmServiceDto(
    Guid Id,
    Guid PlatformId,
    string Name,
    string? Description,
    string DockerName,
    string? DockerServiceId,
    string Spec,
    DateTime? AutoUpdateState_LastCheckedAt,
    string? AutoUpdateState_Status,
    string? AutoUpdateState_CurrentDigest,
    string? AutoUpdateState_RemoteDigest,
    string? AutoUpdateState_LastError,
    string Health,
    string SynchronizationState,
    string ControlState,
    long? ControlStartedAt,
    Guid? ControlTriggeredBy,
    string DesiredSpecHash,
    string? LastAppliedDesiredSpecHash,
    string? LastAppliedRuntimeHash,
    string? AppliedImageDigest,
    long? DockerVersionIndex,
    Guid? OperationId,
    string? OperationKind,
    string? OperationState,
    long? BaseDockerVersion,
    string? TargetDesiredSpecHash,
    string? TargetRuntimeHash,
    long? TargetRowVersion,
    long? ExpectedForceUpdate,
    DateTime? PreparedAt,
    DateTime? AttemptedAt,
    DateTime? CompletedAt,
    long? ObservedDockerVersion,
    string? ResultCode,
    string? Warnings,
    string? ResultMessage,
    string? OperationClusterId,
    Guid? OperationActorId,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    long RowVersion,
    string? Platform_Name = null,
    string? Platform_Status = null,
    string? Platform_Descriptor = null,
    string? TagsJson = null)
{
    public SwarmServiceDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            null,
            string.Empty,
            null,
            string.Empty,
            null,
            null,
            null,
            null,
            null,
            string.Empty,
            string.Empty,
            string.Empty,
            null,
            null,
            string.Empty,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue,
            0)
    {
    }
}

internal sealed class SwarmServicePersistenceParameters
{
    public Guid Id { get; init; }
    public Guid PlatformId { get; init; }
    public required string Name { get; init; }
    public string? Description { get; init; }
    public required string DockerName { get; init; }
    public string? DockerServiceId { get; init; }
    public required string Spec { get; init; }
    public DateTime AutoUpdateState_LastCheckedAt { get; init; }
    public required string AutoUpdateState_Status { get; init; }
    public string? AutoUpdateState_CurrentDigest { get; init; }
    public string? AutoUpdateState_RemoteDigest { get; init; }
    public string? AutoUpdateState_LastError { get; init; }
    public required string Health { get; init; }
    public required string SynchronizationState { get; init; }
    public required string ControlState { get; init; }
    public long? ControlStartedAt { get; init; }
    public Guid? ControlTriggeredBy { get; init; }
    public required string DesiredSpecHash { get; init; }
    public string? LastAppliedDesiredSpecHash { get; init; }
    public string? LastAppliedRuntimeHash { get; init; }
    public string? AppliedImageDigest { get; init; }
    public long? DockerVersionIndex { get; init; }
    public Guid? OperationId { get; init; }
    public string? OperationKind { get; init; }
    public string? OperationState { get; init; }
    public long? BaseDockerVersion { get; init; }
    public string? TargetDesiredSpecHash { get; init; }
    public string? TargetRuntimeHash { get; init; }
    public long? TargetRowVersion { get; init; }
    public long? ExpectedForceUpdate { get; init; }
    public DateTime? PreparedAt { get; init; }
    public DateTime? AttemptedAt { get; init; }
    public DateTime? CompletedAt { get; init; }
    public long? ObservedDockerVersion { get; init; }
    public string? ResultCode { get; init; }
    public string? Warnings { get; init; }
    public string? ResultMessage { get; init; }
    public string? OperationClusterId { get; init; }
    public Guid? OperationActorId { get; init; }
    public Guid CreatedByActorId { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public long RowVersion { get; init; }
    public string TagResourceType { get; init; } = string.Empty;
    public Guid[] TagIds { get; init; } = [];
    public Guid TagCreatedByActorId { get; init; }
}
