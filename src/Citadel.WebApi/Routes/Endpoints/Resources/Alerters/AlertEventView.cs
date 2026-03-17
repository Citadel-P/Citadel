using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertEventView(
    Guid Id,
    Guid AlertRuleId,
    AlertType Type,
    AlertSeverity Severity,
    AlertEventStatus Status,
    string Message,
    AlertEventInfo Info,
    Guid? ResourceId,
    AlertResourceType ResourceType,
    string? ResourcePath,
    Guid CreatedByActorId,
    Guid? AcknowledgedByActorId,
    DateTime? AcknowledgedAt,
    Guid? ResolvedByActorId,
    DateTime? ResolvedAt,
    string? ResolutionNote,
    DateTime CreatedAt,
    DateTime UpdatedAt)
{
    internal static AlertEventView Map(AlertEvent alertEvent)
    {
        return new(
            alertEvent.Id,
            alertEvent.AlertRuleId,
            alertEvent.Type,
            alertEvent.Severity,
            alertEvent.Status,
            alertEvent.Info.HumanMessage,
            alertEvent.Info,
            alertEvent.ResourceId,
            alertEvent.ResourceType,
            BuildResourcePath(alertEvent.ResourceType, alertEvent.ResourceId),
            alertEvent.CreatedByActorId,
            alertEvent.AcknowledgedByActorId,
            alertEvent.AcknowledgedAt,
            alertEvent.ResolvedByActorId,
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            alertEvent.CreatedAt,
            alertEvent.UpdatedAt);
    }

    private static string? BuildResourcePath(AlertResourceType resourceType, Guid? resourceId)
    {
        if (resourceId is null)
            return null;

        return resourceType switch
        {
            AlertResourceType.Platform => $"/platforms/{resourceId}",
            AlertResourceType.Deployment => $"/deployments/{resourceId}",
            AlertResourceType.Stack => $"/stacks/{resourceId}",
            _ => null
        };
    }
}
