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
    string ResourceName,
    AlertResourceType ResourceType,
    string? ResourcePath,
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
            alertEvent.ResourceName,
            alertEvent.ResourceType,
            BuildResourcePath(alertEvent.ResourceType, alertEvent.ResourceId),
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
