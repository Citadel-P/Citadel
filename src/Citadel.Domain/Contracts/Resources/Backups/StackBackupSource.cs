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

public interface ISwarmServiceBackupVolumeResolver
{
    Task<Result<SwarmServiceBackupVolumeResolution>> ResolveAsync(
        Guid swarmServiceId,
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
    bool IsShared,
    string? DockerNodeId = null,
    string? NodeHostname = null);

public sealed record DeploymentBackupVolumeResolution(
    Guid DeploymentId,
    string DeploymentName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<ResolvedStackBackupVolume> Volumes,
    IReadOnlyList<string> Warnings);

public sealed record SwarmServiceBackupVolumeResolution(
    Guid SwarmServiceId,
    string SwarmServiceName,
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
