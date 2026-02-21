using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class AlertEventMappers
{
    internal static AlertEvent ToDomain(this AlertEventDto alertEventDto)
    {
        var info = JsonSerializer.Deserialize(alertEventDto.Info, AlertEventJsonContext.Default.AlertInfo);
        if (info is null)
            throw new InvalidOperationException("Alert info could not be deserialized.");
        if (alertEventDto.ResourceId is null)
            throw new InvalidOperationException("Alert event resource id is required.");

        return AlertEvent.FromPersistence(
            id: alertEventDto.Id,
            alertRuleId: alertEventDto.AlertRuleId,
            type: Enum.Parse<AlertType>(alertEventDto.Type),
            severity: Enum.Parse<AlertSeverity>(alertEventDto.Severity),
            info: info,
            resourceId: alertEventDto.ResourceId.Value,
            resourceType: Enum.Parse<AlertResourceType>(alertEventDto.ResourceType),
            createdByActorId: alertEventDto.CreatedByActorId,
            createdAt: alertEventDto.CreatedAt);
    }
}
