namespace Domain.Contracts.Resources.Stacks;

public sealed record SwarmStackCompatibilityReport(
    bool IsCompatible,
    IReadOnlyList<SwarmStackCompatibilityIssue> Issues);

public sealed record SwarmStackCompatibilityIssue(
    SwarmStackCompatibilitySeverity Severity,
    string Code,
    string Message,
    string? FieldPath = null);
