namespace Infrastructure.Persistence.Dtos;

internal sealed record ActionRunDto(
    Guid Id,
    Guid ActionId,
    string ActionName,
    string Trigger,
    string Status,
    Guid RunAsActorId,
    Guid? TriggeredByActorId,
    string ArgsJson,
    string CodeSnapshot,
    string CodeHash,
    int TimeoutSeconds,
    DateTime QueuedAt,
    DateTime? StartedAt,
    DateTime? FinishedAt,
    long? DurationMs,
    int? ExitCode,
    string? Logs,
    string? ErrorMessage)
{
    public ActionRunDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            string.Empty,
            string.Empty,
            Guid.Empty,
            null,
            "{}",
            string.Empty,
            string.Empty,
            300,
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            null)
    {
    }
}
