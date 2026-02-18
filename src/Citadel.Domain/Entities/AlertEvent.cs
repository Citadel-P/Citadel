using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public class AlertEvent : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid AlertRuleId { get; private set; }
    public AlertType Type { get; private set; }
    public AlertSeverity Severity { get; private set; }
    public AlertInfo Info { get; private set; }
    public Guid? ResourceId { get; private set; }
    public AlertResourceType ResourceType { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public AlertEvent(
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertInfo info,
        Guid? resourceId,
        AlertResourceType resourceType,
        Guid actorId)
    {
        if (info != null && !IsValidInfoForAlert(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        AlertRuleId = alertRuleId;
        Type = type;
        Severity = severity;
        Info = info;
        ResourceId = resourceId;
        ResourceType = resourceType;

        CreatedByActorId = actorId;
        CreatedAt = DateTime.UtcNow;
    }

    private static bool IsValidInfoForAlert(AlertType alertType, AlertInfo alertInfo)
    {
        return (alertType, alertInfo) switch
        {
            (AlertType.PlatformCpuHigh, PlatformCpuHighAlertInfo) => true,
            (AlertType.PlatformRamHigh, PlatformRamHighAlertInfo) => true,
            (AlertType.PlatformVersionMismatch, PlatformVersionMismatchAlertInfo) => true,
            (AlertType.DeploymentImageUpdateAvailable, DeploymentImageUpdateAvailableAlertInfo) => true,
            (AlertType.DeploymentAutoUpdated, DeploymentAutoUpdatedAlertInfo) => true,
            (AlertType.DeploymentFailed, DeploymentFailedAlertInfo) => true,
            (AlertType.StackImageUpdateAvailable, StackImageUpdateAvailableAlertInfo) => true,
            (AlertType.StackAutoUpdated, StackAutoUpdatedAlertInfo) => true,
            (AlertType.StackDeployFailed, StackDeployFailedAlertInfo) => true,

            // Todo: add mappings
            _ => false
        };
    }
}

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(PlatformCpuHighAlertInfo), nameof(AlertType.PlatformCpuHigh))]
[JsonDerivedType(typeof(PlatformRamHighAlertInfo), nameof(AlertType.PlatformRamHigh))]
[JsonDerivedType(typeof(PlatformVersionMismatchAlertInfo), nameof(AlertType.PlatformVersionMismatch))]
[JsonDerivedType(typeof(DeploymentImageUpdateAvailableAlertInfo), nameof(AlertType.DeploymentImageUpdateAvailable))]
[JsonDerivedType(typeof(DeploymentAutoUpdatedAlertInfo), nameof(AlertType.DeploymentAutoUpdated))]
[JsonDerivedType(typeof(DeploymentFailedAlertInfo), nameof(AlertType.DeploymentFailed))]
[JsonDerivedType(typeof(StackImageUpdateAvailableAlertInfo), nameof(AlertType.StackImageUpdateAvailable))]
[JsonDerivedType(typeof(StackAutoUpdatedAlertInfo), nameof(AlertType.StackAutoUpdated))]
[JsonDerivedType(typeof(StackDeployFailedAlertInfo), nameof(AlertType.StackDeployFailed))]
public abstract record AlertInfo;

public record PlatformCpuHighAlertInfo(double CpuUsagePercent) : AlertInfo;
public record PlatformRamHighAlertInfo(double RamUsagePercent) : AlertInfo;
public record PlatformVersionMismatchAlertInfo(string CurrentVersion, string ExpectedVersion) : AlertInfo;
public record DeploymentImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertInfo;
public record DeploymentAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertInfo;
public record DeploymentFailedAlertInfo(string Reason) : AlertInfo;
public record StackImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertInfo;
public record StackAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertInfo;
public record StackDeployFailedAlertInfo(string Reason) : AlertInfo;