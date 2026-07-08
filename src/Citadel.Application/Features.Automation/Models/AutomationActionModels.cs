using Domain.Entities.Automation;

namespace Application.Features.Automation.Models;

public sealed record AutomationActionInputModel(
    string Name,
    string? Description,
    string Code,
    string? DefaultArgsJson,
    bool Enabled,
    bool ScheduleEnabled,
    string? ScheduleCron,
    string? ScheduleTimeZone,
    AutomationWebhookConfig? Webhook,
    int? TimeoutSeconds,
    bool AlertOnFailure,
    Guid? RunAsActorId,
    IReadOnlyCollection<Guid>? TagIds = null);

public sealed record UpdateAutomationActionInputModel(
    string? Description,
    string? Code,
    string? DefaultArgsJson,
    bool? Enabled,
    bool? ScheduleEnabled,
    string? ScheduleCron,
    string? ScheduleTimeZone,
    AutomationWebhookConfig? Webhook,
    int? TimeoutSeconds,
    bool? AlertOnFailure,
    Guid? RunAsActorId);

public sealed record RunAutomationActionInputModel(
    string? ArgsJson,
    int? TimeoutSeconds);

public sealed record TestAutomationActionInputModel(
    string Code,
    string? ArgsJson,
    string? DefaultArgsJson,
    int? TimeoutSeconds,
    Guid? RunAsActorId);

public sealed record AutomationActionResult(AutomationAction Action);

public sealed record AutomationActionListResult(IReadOnlyList<AutomationAction> Actions, IReadOnlyDictionary<Guid, ActionRun> LatestRuns);

public sealed record ActionRunListResult(IReadOnlyList<ActionRun> Runs);
