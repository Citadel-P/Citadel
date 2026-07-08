using System.Security.Cryptography;
using System.Text;

namespace Domain.Entities.Automation;

public sealed class ActionRun(
    Guid actionId,
    string actionName,
    ActionRunTrigger trigger,
    Guid runAsActorId,
    Guid? triggeredByActorId,
    string argsJson,
    string codeSnapshot,
    int timeoutSeconds,
    DateTime? queuedAt = null,
    ActionRunStatus status = ActionRunStatus.Queued,
    DateTime? startedAt = null,
    DateTime? finishedAt = null,
    int? exitCode = null,
    string? logs = null,
    string? errorMessage = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid ActionId { get; private set; } = actionId;
    public string ActionName { get; private set; } = actionName;
    public ActionRunTrigger Trigger { get; private set; } = trigger;
    public ActionRunStatus Status { get; private set; } = status;
    public Guid RunAsActorId { get; private set; } = runAsActorId;
    public Guid? TriggeredByActorId { get; private set; } = triggeredByActorId;
    public string ArgsJson { get; private set; } = string.IsNullOrWhiteSpace(argsJson) ? "{}" : argsJson.Trim();
    public string CodeSnapshot { get; private set; } = codeSnapshot;
    public string CodeHash { get; private set; } = ComputeHash(codeSnapshot);
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public DateTime QueuedAt { get; private set; } = queuedAt ?? DateTime.UtcNow;
    public DateTime? StartedAt { get; private set; } = startedAt;
    public DateTime? FinishedAt { get; private set; } = finishedAt;
    public long? DurationMs { get; private set; } = CalculateDurationMs(startedAt, finishedAt);
    public int? ExitCode { get; private set; } = exitCode;
    public string? Logs { get; private set; } = logs;
    public string? ErrorMessage { get; private set; } = errorMessage;

    public void MarkRunning(DateTime now)
    {
        Status = ActionRunStatus.Running;
        StartedAt = now;
        ErrorMessage = null;
    }

    public void Complete(ActionRunStatus status, int? exitCode, string logs, string? errorMessage, DateTime now)
    {
        Status = status;
        ExitCode = exitCode;
        Logs = logs;
        ErrorMessage = string.IsNullOrWhiteSpace(errorMessage) ? null : errorMessage.Trim();
        FinishedAt = now;
        DurationMs = CalculateDurationMs(StartedAt, FinishedAt);
    }

    public void Reject(string reason, DateTime now)
    {
        Complete(ActionRunStatus.Rejected, null, string.Empty, reason, now);
    }

    public void Cancel(DateTime now)
    {
        Complete(ActionRunStatus.Cancelled, null, Logs ?? string.Empty, "Run cancelled.", now);
    }

    public static ActionRun FromPersistence(
        Guid id,
        Guid actionId,
        string actionName,
        ActionRunTrigger trigger,
        ActionRunStatus status,
        Guid runAsActorId,
        Guid? triggeredByActorId,
        string argsJson,
        string codeSnapshot,
        string codeHash,
        int timeoutSeconds,
        DateTime queuedAt,
        DateTime? startedAt,
        DateTime? finishedAt,
        long? durationMs,
        int? exitCode,
        string? logs,
        string? errorMessage)
    {
        return new ActionRun(
            actionId,
            actionName,
            trigger,
            runAsActorId,
            triggeredByActorId,
            argsJson,
            codeSnapshot,
            timeoutSeconds,
            queuedAt,
            status,
            startedAt,
            finishedAt,
            exitCode,
            logs,
            errorMessage)
        {
            Id = id,
            CodeHash = codeHash,
            DurationMs = durationMs
        };
    }

    private static string ComputeHash(string value)
        => Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(value)));

    private static long? CalculateDurationMs(DateTime? startedAt, DateTime? finishedAt)
        => startedAt.HasValue && finishedAt.HasValue
            ? (long)Math.Max(0, (finishedAt.Value - startedAt.Value).TotalMilliseconds)
            : null;
}
