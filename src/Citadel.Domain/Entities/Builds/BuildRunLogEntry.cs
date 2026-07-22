namespace Domain.Entities.Builds;

public sealed record BuildRunLogEntry(
    Guid Id,
    Guid BuildRunId,
    DateTimeOffset CreatedAt,
    string Stream,
    string Message);
