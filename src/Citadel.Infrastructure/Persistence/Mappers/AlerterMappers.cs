using Domain;
using Domain.Entities;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class AlerterMappers
{
    internal static IEnumerable<AlertRule> ToDomain(this IEnumerable<AlertRuleDto> dto)
        => dto.Select(ToDomain);

    internal static AlertRuleState? ToDomain(this AlertRuleStateDto? dto)
    {
        if (dto is null || dto.ResourceId is null)
            return null;

        return AlertRuleState.FromPersistence(
            alertRuleId: dto.AlertRuleId,
            resourceId: dto.ResourceId.Value,
            consecutiveMatches: dto.ConsecutiveMatches,
            lastTriggeredAt: dto.LastTriggeredAt,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt);
    }

    internal static AlertRule ToDomain(this AlertRuleDto dto)
    {
        return AlertRule.FromPersistence(
            id: dto.Id,
            url: dto.Url,
            type: Enum.Parse<AlertType>(dto.Type),
            cooldownSeconds: dto.CooldownSeconds,
            isEnabled: dto.IsEnabled,
            scope: Enum.Parse<AlertScope>(dto.Scope),
            severity: Enum.Parse<AlertSeverity>(dto.Severity),
            limitedTo: System.Text.Json.JsonSerializer.Deserialize(dto.LimitedTo, AlerterJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo) ?? [],
            quietHours: System.Text.Json.JsonSerializer.Deserialize(dto.QuietHours, AlerterJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour) ?? [],
            requiredMatches: dto.RequiredMatches,
            threshold: dto.Threshold,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt,
            alertRuleState: dto.State_ResourceId != null ? AlertRuleState.FromPersistence(
                alertRuleId: dto.Id,
                resourceId: dto.State_ResourceId.Value,
                consecutiveMatches: dto.State_ConsecutiveMatches != null ? dto.State_ConsecutiveMatches.Value : 3,
                lastTriggeredAt: dto.State_LastTriggeredAt,
                createdByActorId: dto.State_CreatedByActorId != null ? dto.State_CreatedByActorId.Value : Constants.SystemId,
                createdAt: dto.State_CreatedAt != null ? dto.State_CreatedAt.Value : DateTime.UtcNow
            ) : null
        );
    }
}
