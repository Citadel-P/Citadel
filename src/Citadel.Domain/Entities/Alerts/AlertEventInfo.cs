using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Alerts;

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
public abstract record AlertEventInfo;

public record PlatformCpuHighAlertInfo(double CpuUsagePercent) : AlertEventInfo;
public record PlatformRamHighAlertInfo(double RamUsagePercent) : AlertEventInfo;
public record PlatformVersionMismatchAlertInfo(string CurrentVersion, string ExpectedVersion) : AlertEventInfo;
public record DeploymentImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertEventInfo;
public record DeploymentAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertEventInfo;
public record DeploymentFailedAlertInfo(string Reason) : AlertEventInfo;
public record StackImageUpdateAvailableAlertInfo(string CurrentImage, string LatestImage) : AlertEventInfo;
public record StackAutoUpdatedAlertInfo(string PreviousImage, string UpdatedImage) : AlertEventInfo;
public record StackDeployFailedAlertInfo(string Reason) : AlertEventInfo;
