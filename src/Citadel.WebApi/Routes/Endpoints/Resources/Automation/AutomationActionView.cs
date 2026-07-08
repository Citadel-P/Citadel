using Application.Features.Automation.Models;
using Domain;
using Domain.Entities.Automation;

namespace WebApi.Routes.Endpoints.Resources.Automation;

public sealed record AutomationActionView(
    Guid Id,
    string Name,
    string? Description,
    string Code,
    string DefaultArgsJson,
    bool Enabled,
    bool ScheduleEnabled,
    string? ScheduleCron,
    string ScheduleTimeZone,
    AutomationWebhookConfig? Webhook,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId,
    DateTime? LastScheduledRunAt,
    ResourceControlState ControlState,
    Guid? CurrentRunId,
    long RowVersion,
    AutomationActionRunView? LatestRun,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt)
{
    internal static AutomationActionView Map(AutomationActionResult result)
        => Map(result.Action, null);

    internal static AutomationActionView Map(AutomationAction action)
        => Map(action, null);

    internal static AutomationActionView Map(AutomationAction action, ActionRun? latestRun)
        => new(
            action.Id,
            action.Name,
            action.Description,
            action.Code,
            action.DefaultArgsJson,
            action.Enabled,
            action.ScheduleEnabled,
            action.ScheduleCron,
            action.ScheduleTimeZone,
            action.Webhook,
            action.TimeoutSeconds,
            action.AlertOnFailure,
            action.RunAsActorId,
            action.LastScheduledRunAt,
            action.ControlState,
            action.CurrentRunId,
            action.RowVersion,
            latestRun is null ? null : AutomationActionRunView.Map(latestRun, includeLogs: false, includeCode: false),
            action.CreatedByActorId,
            action.CreatedAt,
            action.UpdatedAt);
}

public sealed record AutomationActionsView(IReadOnlyList<AutomationActionView> Actions)
{
    internal static AutomationActionsView Map(AutomationActionListResult result)
        => new([.. result.Actions.Select(action =>
            AutomationActionView.Map(action, result.LatestRuns.GetValueOrDefault(action.Id)))]);
}

public sealed record AutomationActionRunView(
    Guid Id,
    Guid ActionId,
    string ActionName,
    ActionRunTrigger Trigger,
    ActionRunStatus Status,
    Guid RunAsActorId,
    Guid? TriggeredByActorId,
    string ArgsJson,
    string? CodeSnapshot,
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
    internal static AutomationActionRunView Map(ActionRun run)
        => Map(run, includeLogs: true, includeCode: true);

    internal static AutomationActionRunView Map(ActionRun run, bool includeLogs, bool includeCode)
        => new(
            run.Id,
            run.ActionId,
            run.ActionName,
            run.Trigger,
            run.Status,
            run.RunAsActorId,
            run.TriggeredByActorId,
            run.ArgsJson,
            includeCode ? run.CodeSnapshot : null,
            run.CodeHash,
            run.TimeoutSeconds,
            run.QueuedAt,
            run.StartedAt,
            run.FinishedAt,
            run.DurationMs,
            run.ExitCode,
            includeLogs ? run.Logs : null,
            run.ErrorMessage);
}

public sealed record AutomationActionRunsView(IReadOnlyList<AutomationActionRunView> Runs)
{
    internal static AutomationActionRunsView Map(ActionRunListResult result)
        => new([.. result.Runs.Select(run => AutomationActionRunView.Map(run, includeLogs: false, includeCode: false))]);
}

public sealed record AutomationActionRunLogsView(Guid RunId, string Logs)
{
    internal static AutomationActionRunLogsView Map(ActionRun run)
        => new(run.Id, run.Logs ?? string.Empty);
}

public sealed record AutomationActionInput(
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
    Guid? RunAsActorId)
{
    internal AutomationActionInputModel ToModel()
        => new(
            Name,
            Description,
            Code,
            DefaultArgsJson,
            Enabled,
            ScheduleEnabled,
            ScheduleCron,
            ScheduleTimeZone,
            Webhook,
            TimeoutSeconds,
            AlertOnFailure,
            RunAsActorId);
}

public sealed record UpdateAutomationActionInput(
    string? Description = null,
    string? Code = null,
    string? DefaultArgsJson = null,
    bool? Enabled = null,
    bool? ScheduleEnabled = null,
    string? ScheduleCron = null,
    string? ScheduleTimeZone = null,
    AutomationWebhookConfig? Webhook = null,
    int? TimeoutSeconds = null,
    bool? AlertOnFailure = null,
    Guid? RunAsActorId = null)
{
    internal UpdateAutomationActionInputModel ToModel()
        => new(
            Description,
            Code,
            DefaultArgsJson,
            Enabled,
            ScheduleEnabled,
            ScheduleCron,
            ScheduleTimeZone,
            Webhook,
            TimeoutSeconds,
            AlertOnFailure,
            RunAsActorId);
}

public sealed record RunAutomationActionInput(string? ArgsJson, int? TimeoutSeconds)
{
    internal RunAutomationActionInputModel ToModel() => new(ArgsJson, TimeoutSeconds);
}

public sealed record TestAutomationActionInput(
    string Code,
    string? ArgsJson,
    string? DefaultArgsJson,
    int? TimeoutSeconds,
    Guid? RunAsActorId)
{
    internal TestAutomationActionInputModel ToModel()
        => new(Code, ArgsJson, DefaultArgsJson, TimeoutSeconds, RunAsActorId);
}
