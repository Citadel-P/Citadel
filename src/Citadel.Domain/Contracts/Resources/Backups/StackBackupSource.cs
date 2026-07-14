using Domain.Entities.Platforms;
using LightResults;

namespace Domain.Contracts.Resources.Backups;

public interface IStackBackupVolumeResolver
{
    Task<Result<StackBackupVolumeResolution>> ResolveAsync(
        Guid stackId,
        CancellationToken cancellationToken);
}

public interface IDeploymentBackupVolumeResolver
{
    Task<Result<DeploymentBackupVolumeResolution>> ResolveAsync(
        Guid deploymentId,
        CancellationToken cancellationToken);
}

public sealed record StackBackupVolumeResolution(
    Guid StackId,
    string StackName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<ResolvedStackBackupVolume> Volumes,
    IReadOnlyList<string> Warnings);

public sealed record ResolvedStackBackupVolume(
    Guid PlatformId,
    string VolumeName,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared);

public sealed record DeploymentBackupVolumeResolution(
    Guid DeploymentId,
    string DeploymentName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<ResolvedStackBackupVolume> Volumes,
    IReadOnlyList<string> Warnings);

public enum StackVolumeKind
{
    DeclaredNamed,
    ExternalNamed,
    AnonymousNamed
}
