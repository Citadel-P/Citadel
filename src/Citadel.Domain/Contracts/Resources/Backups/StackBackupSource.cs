using Domain.Entities.Platforms;
using LightResults;

namespace Domain.Contracts.Resources.Backups;

public interface IStackBackupVolumeResolver
{
    Task<Result<StackBackupVolumeResolution>> ResolveAsync(
        Guid stackId,
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

public enum StackVolumeKind
{
    DeclaredNamed,
    ExternalNamed,
    AnonymousNamed
}
