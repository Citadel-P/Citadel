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
[JsonDerivedType(typeof(DeploymentFailedAlertInfo), nameof(AlertType.DeploymentAutoDeployFailed))]
[JsonDerivedType(typeof(StackImageUpdateAvailableAlertInfo), nameof(AlertType.StackImageUpdateAvailable))]
[JsonDerivedType(typeof(StackAutoUpdatedAlertInfo), nameof(AlertType.StackAutoUpdated))]
[JsonDerivedType(typeof(StackDeployFailedAlertInfo), nameof(AlertType.StackAutoDeployFailed))]
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

public record PlatformCpuHighAlertInfo(double CpuUsagePercent) : AlertEventInfo
{
    public override string HumanMessage => $"CPU usage is high: {CpuUsagePercent:0.##}%";
}

public record PlatformRamHighAlertInfo(double RamUsagePercent) : AlertEventInfo
{
    public override string HumanMessage => $"RAM usage is high: {RamUsagePercent:0.##}%";
}

public record PlatformVersionMismatchAlertInfo(string CurrentAgentVersion, string ExpectedAgentVersion) : AlertEventInfo
{
    public override string HumanMessage =>
        $"Agent version mismatch: current {CurrentAgentVersion}, expected {ExpectedAgentVersion}";
}

public record PlatformUnreachableAlertInfo(string Name, string Address) : AlertEventInfo
{
    public override string HumanMessage => $"Platform '{Name}' at {Address} is unreachable";
}

public record UnmanagedContainerCreatedAlertInfo(string PlatformName, string PlatformAddress, string ContainerName) : AlertEventInfo
{
    public override string HumanMessage =>
        $"Unmanaged container '{ContainerName}' was created on platform '{PlatformName}' ({PlatformAddress})";
}

public record DeploymentImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertEventInfo
{
    public override string HumanMessage => $"New image available for deployment: {CurrentImage} → {LatestImage}";
}

public record DeploymentAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertEventInfo
{
    public override string HumanMessage => $"Deployment auto-updated: {PreviousImage} → {UpdatedImage}";
}

public record DeploymentFailedAlertInfo(string Reason) : AlertEventInfo
{
    public override string HumanMessage => $"Deployment failed: {Reason}";
}

public record StackImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertEventInfo
{
    public override string HumanMessage => $"New image available for stack: {CurrentImage} → {LatestImage}";
}

public record StackAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertEventInfo
{
    public override string HumanMessage => $"Stack auto-updated: {PreviousImage} → {UpdatedImage}";
}

public record StackDeployFailedAlertInfo(string Reason) : AlertEventInfo
{
    public override string HumanMessage => $"Stack deployment failed: {Reason}";
}
