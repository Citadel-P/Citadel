namespace Domain.Contracts.Resources.Automation;

public sealed record AutomationActionRunStreamItem(
    Guid? RunId = null,
    string? Status = null,
    string? Stream = null,
    string? ProgressMessage = null,
    string? ErrorMessage = null,
    AutomationActionRunStreamError? Error = null);

public sealed record AutomationActionRunStreamError(long? Code, string? Message);
