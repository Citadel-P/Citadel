using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Domain.Entities.Alerts;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(PlatformCpuHighAlertInfo), nameof(AlertType.PlatformCpuHigh))]
[JsonDerivedType(typeof(PlatformRamHighAlertInfo), nameof(AlertType.PlatformRamHigh))]
[JsonDerivedType(typeof(PlatformUnreachableAlertInfo), nameof(AlertType.PlatformUnreachable))]
[JsonDerivedType(typeof(UnmanagedContainerCreatedAlertInfo), nameof(AlertType.UnmanagedContainerCreated))]
[JsonDerivedType(typeof(PlatformVersionMismatchAlertInfo), nameof(AlertType.PlatformVersionMismatch))]
[JsonDerivedType(typeof(DeploymentImageUpdateAvailableAlertInfo), nameof(AlertType.DeploymentImageUpdateAvailable))]
[JsonDerivedType(typeof(DeploymentAutoUpdatedAlertInfo), nameof(AlertType.DeploymentAutoUpdated))]
[JsonDerivedType(typeof(DeploymentAutoDeployFailedAlertInfo), nameof(AlertType.DeploymentAutoDeployFailed))]
[JsonDerivedType(typeof(StackImageUpdateAvailableAlertInfo), nameof(AlertType.StackImageUpdateAvailable))]
[JsonDerivedType(typeof(StackAutoUpdatedAlertInfo), nameof(AlertType.StackAutoUpdated))]
[JsonDerivedType(typeof(StackDeployFailedAlertInfo), nameof(AlertType.StackAutoDeployFailed))]
[JsonDerivedType(typeof(StackServiceAutoUpdatedAlertInfo), nameof(AlertType.StackServiceAutoUpdated))]
[JsonDerivedType(typeof(StackServiceAutoDeployFailedAlertInfo), nameof(AlertType.StackServiceAutoDeployFailed))]
[JsonDerivedType(typeof(StackDriftDetectedAlertInfo), nameof(AlertType.StackDriftDetected))]
public abstract record AlertEventInfo
{
    /// <summary>
    /// Human-readable message for notifications.
    /// Override in derived records for custom messages.
    /// Defaults to JSON serialization if not overridden.
    /// </summary>
    public virtual string HumanMessage =>
        JsonSerializer.Serialize(this, AlertEventJsonContext.Default.AlertEventInfo);
}

public record PlatformCpuHighAlertInfo(string PlatformName, double CpuUsagePercent) : AlertEventInfo
{
    public override string HumanMessage => $"'{PlatformName}' CPU usage is high: {CpuUsagePercent:0.##}%";
}

public record PlatformRamHighAlertInfo(string PlatformName, double RamUsagePercent) : AlertEventInfo
{
    public override string HumanMessage => $"'{PlatformName}' RAM usage is high: {RamUsagePercent:0.##}%";
}

public record PlatformVersionMismatchAlertInfo(string PlatformName, string CurrentAgentVersion, string ExpectedAgentVersion) : AlertEventInfo
{
    public override string HumanMessage =>
        $"'{PlatformName}' Agent version mismatch: current {CurrentAgentVersion}, expected {ExpectedAgentVersion}";
}

public record PlatformUnreachableAlertInfo(string PlatformName, Guid Id, string Address) : AlertEventInfo
{
    public override string HumanMessage => $"Platform '{PlatformName}' with id '{Id}' at {Address} is unreachable";
}

public record UnmanagedContainerCreatedAlertInfo(string PlatformName, string PlatformAddress, string ContainerName, string ContainerId) : AlertEventInfo
{
    public override string HumanMessage =>
        $"Unmanaged container '{ContainerName}' was created on platform '{PlatformName}' ({PlatformAddress})";
}

public record DeploymentImageUpdateAvailableAlertInfo(string DeploymentName, string CurrentImage, string LatestImage) : AlertEventInfo
{
    public override string HumanMessage => $"New image available for deployment '{DeploymentName}': {CurrentImage} → {LatestImage}";
}

public record DeploymentAutoUpdatedAlertInfo(string DeploymentName, string PreviousImage, string UpdatedImage) : AlertEventInfo
{
    public override string HumanMessage => $"Deployment '{DeploymentName}' auto-updated: {PreviousImage} → {UpdatedImage}";
}

public record DeploymentAutoDeployFailedAlertInfo(string DeploymentName, string Reason) : AlertEventInfo
{
    public override string HumanMessage => $"Deployment '{DeploymentName}' failed: {Reason}";
}

public sealed record StackImageUpdateItem(
    string ServiceName,
    string ImageName,
    string CurrentDigest,
    string LatestDigest);

public record StackImageUpdateAvailableAlertInfo(string StackName, IReadOnlyList<StackImageUpdateItem> Updates) : AlertEventInfo
{
    public override string HumanMessage => StackAlertMessageFormatter.FormatStackUpdateMessage($"New image available for stack '{StackName}'", Updates);
}

public record StackAutoUpdatedAlertInfo(string StackName, IReadOnlyList<StackImageUpdateItem> Updates) : AlertEventInfo
{
    public override string HumanMessage => StackAlertMessageFormatter.FormatStackUpdateMessage($"Stack '{StackName}' auto-updated", Updates);
}

public record StackServiceAutoUpdatedAlertInfo(string StackName, IReadOnlyList<StackImageUpdateItem> Updates) : AlertEventInfo
{
    public override string HumanMessage => StackAlertMessageFormatter.FormatStackUpdateMessage($"Stack '{StackName}' services auto-updated", Updates);
}

public record StackDeployFailedAlertInfo(string StackName, string Reason) : AlertEventInfo
{
    public override string HumanMessage => $"Stack '{StackName}' deployment failed: {Reason}";
}

public record StackServiceAutoDeployFailedAlertInfo(string StackName, IReadOnlyList<string> ServiceNames, string Reason) : AlertEventInfo
{
    public override string HumanMessage
    {
        get
        {
            var services = ServiceNames.Count == 0 ? "selected services" : string.Join(", ", ServiceNames);
            return $"Stack '{StackName}' service deployment failed for {services}: {Reason}";
        }
    }
}

public sealed record StackDriftDetectedAlertInfo(
    Guid StackId,
    string StackName,
    Guid PlatformId,
    string PlatformName,
    int DriftCount,
    bool HasAutoFixableDrift,
    bool HasStructuralDrift,
    IReadOnlyList<string> DriftSummaries) : AlertEventInfo
{
    public override string HumanMessage
    {
        get
        {
            var issueText = DriftCount == 1 ? "1 issue found" : $"{DriftCount} issues found";
            var actionText = HasStructuralDrift
                ? " Re-apply is required."
                : HasAutoFixableDrift
                    ? " Safe reconciliation is available."
                    : string.Empty;

            return $"Drift detected on stack '{StackName}': {issueText}.{actionText}";
        }
    }
}

file static class StackAlertMessageFormatter
{
    public static string FormatStackUpdateMessage(string prefix, IReadOnlyList<StackImageUpdateItem> updates)
    {
        if (updates.Count == 0)
            return prefix;

        var suffix = string.Join(
            ", ",
            updates.Select(update => $"{update.ServiceName}: {update.CurrentDigest} → {update.LatestDigest}"));

        return $"{prefix}: {suffix}";
    }
}
