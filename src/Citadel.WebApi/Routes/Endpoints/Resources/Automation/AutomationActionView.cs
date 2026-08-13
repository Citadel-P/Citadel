using Application.Features.Automation.Models;
using Application.Permissions;
using Domain;
using Domain.Entities.Automation;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;

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
    DateTime UpdatedAt,
    IReadOnlyList<TagSummaryView> Tags,
    ResourceCapabilities? Capabilities = null)
{
    internal static AutomationActionView Map(AutomationActionResult result)
        => Map(result.Action, (ActionRun?)null);

    internal static Task<AutomationActionView> Map(AutomationActionResult result, IPermissionEvaluator permissionEvaluator)
        => Map(result.Action, permissionEvaluator);

    internal static AutomationActionView Map(AutomationAction action)
        => Map(action, (ActionRun?)null);

    internal static async Task<AutomationActionView> Map(AutomationAction action, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(action.Id, ResourceType.AutomationAction);
        return Map(action) with
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }

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
            action.UpdatedAt,
            [.. action.Tags.Select(TagSummaryView.Map)]);
}

public sealed record AutomationActionsView(IReadOnlyList<AutomationActionView> Actions, ResourceCapabilities Capabilities)
{
    internal static async Task<AutomationActionsView> Map(
        AutomationActionListResult result,
        IPermissionEvaluator permissionEvaluator)
    {
        var actions = result.Actions as AutomationAction[] ?? [.. result.Actions];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.AutomationAction);

        if (actions.Length == 0)
            return new AutomationActionsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = new Guid[actions.Length];

        for (var i = 0; i < actions.Length; i++)
        {
            ids[i] = actions[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.AutomationAction);
        var views = new AutomationActionView[actions.Length];

        for (var i = 0; i < actions.Length; i++)
        {
            var action = actions[i];
            var baseView = AutomationActionView.Map(action, result.LatestRuns.GetValueOrDefault(action.Id));

            perms.TryGetValue(action.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new AutomationActionsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
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
    Guid? RunAsActorId,
    IReadOnlyCollection<Guid>? TagIds = null)
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
            RunAsActorId,
            TagIds);
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

public sealed record TestAutomationActionInput(string? ArgsJson)
{
    internal TestAutomationActionInputModel ToModel()
        => new(ArgsJson);
}
