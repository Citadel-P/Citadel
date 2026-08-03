namespace Infrastructure.Persistence.Dtos;

internal sealed record DeploymentDto(
    Guid Id,
    string Name,
    string Status,
    long RowVersion,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    Guid PlatformId,
    string ControlState,
    long? ControlStartedAt,
    Guid? ControlTriggeredBy,
    DateTime AutoUpdateState_LastCheckedAt,
    string AutoUpdateState_Status,
    string? AutoUpdateState_CurrentDigest,
    string? AutoUpdateState_RemoteDigest,
    string? AutoUpdateState_LastError,
    string? Spec = null,
    string? Description = null,
    string? Platform_Name = null,
    string? Platform_Status = null,
    string? Image_Name = null,
    string? Image_DockerImageId = null,
    Guid? Image_Id = null,
    Guid? Container_ContainerId = null,
    string? Container_DockerContainerId = null,
    string? Container_DockerImageId = null,
    Guid? ActivityEvent_Id = null,
    string? ActivityEvent_Status = null,
    string? ActivityEvent_EventType = null,
    string? ActivityEvent_ActivityEventInfo = null,
    DateTime? ActivityEvent_CreatedAt = null,
    string? TagsJson = null,
    string? Platform_Descriptor = null
    )
{
    public DeploymentDto()
        : this(
            Guid.Empty,
            string.Empty,
            string.Empty,
            0,
            DateTime.MinValue,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            null,
            null,
            DateTime.MinValue,
            string.Empty,
            null,
            null,
            null)
    {
    }
}
