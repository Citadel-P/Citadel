using Domain;
using Domain.Entities.Alerts;
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
            name: dto.Name,
            description: dto.Description,
            type: Enum.Parse<AlertType>(dto.Type),
            cooldownSeconds: dto.CooldownSeconds,
            status: Enum.Parse<AlertRuleStatus>(dto.Status),
            severity: Enum.Parse<AlertSeverity>(dto.Severity),
            channelIds: dto.ChannelIds != null && string.Compare(dto.ChannelIds, "[null]", true) != 0 ? System.Text.Json.JsonSerializer.Deserialize(dto.ChannelIds, DeploymentJsonContext.Default.IEnumerableGuid) : [],
            limitedTo: System.Text.Json.JsonSerializer.Deserialize(dto.LimitedTo, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo) ?? [],
            quietHours: System.Text.Json.JsonSerializer.Deserialize(dto.QuietHours, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour) ?? [],
            requiredMatches: dto.RequiredMatches,
            threshold: dto.Threshold,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt
        );
    }
}
