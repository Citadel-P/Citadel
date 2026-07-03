namespace Infrastructure.Persistence.Dtos;

internal sealed record GitRepositoryDto(
    Guid Id,
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    string Status,
    string SyncMode,
    int? SyncIntervalMinutes,
    Guid? GitAccountId,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string? Webhook,
    string? OnClone,
    string? OnPull,
    string? ControlState,
    long? ControlStartedAt,
    Guid? ControlTriggeredBy,
    long RowVersion,
    string? GitAccount_Name = null,
    string? GitAccount_Domain = null,
    string? GitAccount_Transport = null,
    string? GitAccount_AuthType = null,
    string? GitAccount_Configuration = null,
    Guid? ActivityEvent_Id = null,
    string? ActivityEvent_Status = null,
    string? ActivityEvent_EventType = null,
    string? ActivityEvent_ActivityEventInfo = null,
    DateTime? ActivityEvent_CreatedAt = null,
    string? TagsJson = null)
{
    public GitRepositoryDto()
        : this(
            Guid.Empty,
            string.Empty,
            null,
            string.Empty,
            string.Empty,
            string.Empty,
            string.Empty,
            null,
            null,
            DateTime.MinValue,
            Guid.Empty,
            null,
            null,
            null,
            null,
            null,
            null,
            0)
    {
    }
}
