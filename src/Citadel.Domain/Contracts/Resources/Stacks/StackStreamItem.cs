namespace Domain.Contracts.Resources.Stacks;

public sealed record StackStreamItem(
    StackApplyEventType Type,
    DateTimeOffset Timestamp,
    string? Message = null,
    int? ExitCode = null)
{
    public static StackStreamItem FromStdOut(string message) =>
        new(StackApplyEventType.StdOut, DateTimeOffset.UtcNow, message);

    public static StackStreamItem FromStdErr(string message) =>
        new(StackApplyEventType.StdErr, DateTimeOffset.UtcNow, message);

    public static StackStreamItem SystemMessage(string message) =>
        new(StackApplyEventType.SystemMessage, DateTimeOffset.UtcNow, message);

    public static StackStreamItem Finished(int exitCode) =>
        new(StackApplyEventType.CommandCompleted, DateTimeOffset.UtcNow, null, exitCode);
}
