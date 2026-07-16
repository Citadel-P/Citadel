namespace Domain.Contracts.Resources.Stacks;

public sealed record StackApplyResult(
    StackApplyEventType Type,
    string? Message = null,
    int? ExitCode = null,
    StackReleaseStatus? StackStatus = null)
{
    public static StackApplyResult StdOut(string message) => new(StackApplyEventType.StdOut, message);

    public static StackApplyResult StdErr(string message) => new(StackApplyEventType.StdErr, message);

    public static StackApplyResult SystemMessage(string message) => new(StackApplyEventType.SystemMessage, message);

    public static StackApplyResult Finished(int exitCode) => new(StackApplyEventType.CommandCompleted, ExitCode: exitCode);

    public static StackApplyResult ComposeStatus(StackReleaseStatus status) => new(StackApplyEventType.SystemMessage, StackStatus: status);
}
