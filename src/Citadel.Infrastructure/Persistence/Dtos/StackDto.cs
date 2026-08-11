namespace Infrastructure.Persistence.Dtos;

internal sealed record StackDto(
    Guid Id,
    Guid CurrentStackReleaseId,
    string Name,
    string StackSource,
    string StackUpdateState,
    string? DriftPolicy,
    long RowVersion,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string ControlState,
    long? ControlStartedAt,
    Guid? ControlTriggeredBy,
    string? Description = null,
    Guid? CurrentRelease_Id = null,
    Guid? CurrentRelease_StackId = null,
    Guid? CurrentRelease_PlatformId = null,
    string? CurrentRelease_Status = null,
    string? CurrentRelease_Version = null,
    string? CurrentRelease_Spec = null,
    string? CurrentRelease_Source = null,
    string? CurrentRelease_ResourceBindings = null,
    DateTime? CurrentRelease_CreatedAt = null,
    Guid? CurrentRelease_CreatedByActorId = null,
    string? Platform_Name = null,
    string? Platform_Status = null,
    Guid? ActivityEvent_Id = null,
    string? ActivityEvent_Status = null,
    string? ActivityEvent_EventType = null,
    string? ActivityEvent_ActivityEventInfo = null,
    DateTime? ActivityEvent_CreatedAt = null,
    string? TagsJson = null,
    string? Platform_Descriptor = null,
    string? Platform_ClusterId = null,
    string? Platform_Address = null,
    string? Platform_ConnectorType = null
    )
{
    public StackDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            string.Empty,
            string.Empty,
            null,
            0,
            DateTime.MinValue,
            Guid.Empty,
            string.Empty,
            null,
            null)
    {
    }

    internal bool HasCurrentReleaseIdentity
        => CurrentRelease_Id != null
        && CurrentRelease_PlatformId != null
        && CurrentRelease_Status != null
        && CurrentRelease_Version != null
        && CurrentRelease_CreatedAt != null
        && CurrentRelease_CreatedByActorId != null;
}
