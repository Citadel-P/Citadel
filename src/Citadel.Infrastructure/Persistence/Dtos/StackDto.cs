namespace Infrastructure.Persistence.Dtos;

internal sealed record StackDto(
    Guid Id,
    Guid CurrentStackReleaseId,
    string Name,
    string StackSource,
    string StackUpdateState,
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
    DateTime? CurrentRelease_CreatedAt = null,
    Guid? CurrentRelease_CreatedByActorId = null,
    string? Platform_Name = null,
    string? Platform_Status = null
    )
{
    internal bool HasCurrentReleaseIdentity
        => CurrentRelease_Id != null
        && CurrentRelease_PlatformId != null
        && CurrentRelease_Status != null
        && CurrentRelease_Version != null
        && CurrentRelease_CreatedAt != null
        && CurrentRelease_CreatedByActorId != null;
}
