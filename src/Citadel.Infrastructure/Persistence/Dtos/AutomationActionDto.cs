namespace Infrastructure.Persistence.Dtos;

internal sealed record AutomationActionDto(
    Guid Id,
    string Name,
    string? Description,
    string Code,
    string DefaultArgsJson,
    bool Enabled,
    bool ScheduleEnabled,
    string? ScheduleCron,
    string ScheduleTimeZone,
    string? Webhook,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId,
    DateTime? LastScheduledRunAt,
    string? ControlState,
    Guid? CurrentRunId,
    long RowVersion,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt)
{
    public AutomationActionDto()
        : this(
            Guid.Empty,
            string.Empty,
            null,
            string.Empty,
            "{}",
            false,
            false,
            null,
            "UTC",
            null,
            300,
            false,
            Guid.Empty,
            null,
            null,
            null,
            0,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue)
    {
    }
}
