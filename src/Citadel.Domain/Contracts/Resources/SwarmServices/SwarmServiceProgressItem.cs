namespace Domain.Contracts.Resources.SwarmServices;

public sealed record SwarmServiceProgressItem(
    Guid ServiceId,
    Guid? OperationId,
    string Stage,
    string Message,
    bool IsCompleted = false,
    bool IsWarning = false,
    string? ErrorMessage = null);
