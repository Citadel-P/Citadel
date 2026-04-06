using Domain;
using Domain.Entities.Alerts;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class AlertEventMappers
{
    internal static AlertEvent ToDomain(this AlertEventDto alertEventDto)
    {
        var info = JsonSerializer.Deserialize(alertEventDto.Info, AlertEventJsonContext.Default.AlertEventInfo);
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
            resourceName: alertEventDto.ResourceName,
            resourceType: Enum.Parse<AlertResourceType>(alertEventDto.ResourceType),
            deduplicationKey: alertEventDto.DeduplicationKey,
            openIncidentKey: alertEventDto.OpenIncidentKey,
            acknowledgedByActorId: alertEventDto.AcknowledgedByActorId,
            acknowledgedAt: alertEventDto.AcknowledgedAt,
            resolvedByActorId: alertEventDto.ResolvedByActorId,
            resolvedAt: alertEventDto.ResolvedAt,
            resolutionNote: alertEventDto.ResolutionNote,
            createdAt: alertEventDto.CreatedAt,
            updatedAt: alertEventDto.UpdatedAt,
            actor: alertEventDto.Actor_Id is null
                ? null
                : Actor.FromPersistence(
                    id: alertEventDto.Actor_Id ?? alertEventDto.ResolvedByActorId ?? alertEventDto.AcknowledgedByActorId ?? Guid.Empty,
                    type: alertEventDto.Actor_Type is null ? ActorType.System : Enum.Parse<ActorType>(alertEventDto.Actor_Type),
                    actorMetadata: new ActorMetadata(alertEventDto.Actor_Name ?? "System"),
                    isEnabled: true));
    }
}
