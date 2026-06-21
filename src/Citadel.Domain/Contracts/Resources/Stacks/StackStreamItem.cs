namespace Domain.Contracts.Resources.Stacks;

public sealed record StackStreamItem(
    StackApplyEventType Type,
    DateTimeOffset Timestamp,
    string? ProgressMessage = null,
    string? Message = null, // error message
    int? ExitCode = null)
{
    public static StackStreamItem FromStdOut(string message) =>
        new(StackApplyEventType.StdOut, DateTimeOffset.UtcNow, ProgressMessage: message);

    public static StackStreamItem FromStdErr(string message, int exitCode) =>
        new(StackApplyEventType.StdErr, DateTimeOffset.UtcNow, Message: message, ExitCode: exitCode);

    public static StackStreamItem SystemMessage(string message, int exitCode) =>
        new(StackApplyEventType.SystemMessage, DateTimeOffset.UtcNow, ProgressMessage: message, ExitCode: exitCode);

    public static StackStreamItem Finished(int exitCode) =>
        new(StackApplyEventType.CommandCompleted, DateTimeOffset.UtcNow, null, ExitCode: exitCode);
}
