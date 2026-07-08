using Domain;
using Domain.Entities.Automation;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class AutomationMappers
{
    internal static AutomationAction ToDomain(this AutomationActionDto dto)
    {
        var action = AutomationAction.FromPersistence(
            dto.Id,
            dto.Name,
            dto.Description,
            dto.Code,
            dto.DefaultArgsJson,
            dto.Enabled,
            dto.ScheduleEnabled,
            dto.ScheduleCron,
            dto.ScheduleTimeZone,
            string.IsNullOrWhiteSpace(dto.Webhook)
                ? null
                : JsonSerializer.Deserialize(dto.Webhook, AutomationJsonContext.Default.AutomationWebhookConfig),
            dto.TimeoutSeconds,
            dto.AlertOnFailure,
            dto.RunAsActorId,
            dto.LastScheduledRunAt,
            string.IsNullOrWhiteSpace(dto.ControlState) ? ResourceControlState.Idle : Enum.Parse<ResourceControlState>(dto.ControlState),
            dto.CurrentRunId,
            dto.RowVersion,
            dto.CreatedByActorId,
            dto.CreatedAt,
            dto.UpdatedAt);

        action.AssignTags(dto.TagsJson.ToTagSummaries());
        return action;
    }

    internal static IEnumerable<AutomationAction> ToDomain(this IEnumerable<AutomationActionDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static ActionRun ToDomain(this ActionRunDto dto)
        => ActionRun.FromPersistence(
            dto.Id,
            dto.ActionId,
            dto.ActionName,
            Enum.Parse<ActionRunTrigger>(dto.Trigger),
            Enum.Parse<ActionRunStatus>(dto.Status),
            dto.RunAsActorId,
            dto.TriggeredByActorId,
            dto.ArgsJson,
            dto.CodeSnapshot,
            dto.CodeHash,
            dto.TimeoutSeconds,
            dto.QueuedAt,
            dto.StartedAt,
            dto.FinishedAt,
            dto.DurationMs,
            dto.ExitCode,
            dto.Logs,
            dto.ErrorMessage);

    internal static IEnumerable<ActionRun> ToDomain(this IEnumerable<ActionRunDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());
}
