using System.Text.Json.Serialization;

namespace Domain.Entities.Backups;

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(DockerVolumeBackupSource), "DockerVolume")]
[JsonDerivedType(typeof(CitadelSystemBackupSource), "CitadelSystem")]
[JsonDerivedType(typeof(StackBackupSource), "Stack")]
[JsonDerivedType(typeof(DeploymentBackupSource), "Deployment")]
[JsonDerivedType(typeof(SwarmServiceBackupSource), "SwarmService")]
public abstract record BackupSourceSpec
{
    public abstract BackupSourceType Type { get; }
    public abstract string StableKey { get; }
}

public sealed record DockerVolumeBackupSource(
    Guid PlatformId,
    string VolumeName,
    VolumeBackupConsistency Consistency = VolumeBackupConsistency.Live,
    string? DockerNodeId = null) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.DockerVolume;
    public override string StableKey => string.IsNullOrWhiteSpace(DockerNodeId)
        ? $"{PlatformId}:{VolumeName}"
        : $"{PlatformId}:{DockerNodeId.Trim()}:{VolumeName}";
}

public sealed record CitadelSystemBackupSource() : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.CitadelSystem;
    public override string StableKey => "citadel-system";
}

public sealed record StackBackupSource(Guid StackId) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.Stack;
    public override string StableKey => $"stack:{StackId}";
}

public sealed record DeploymentBackupSource(Guid DeploymentId) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.Deployment;
    public override string StableKey => $"deployment:{DeploymentId}";
}

public sealed record SwarmServiceBackupSource(Guid SwarmServiceId) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.SwarmService;
    public override string StableKey => $"swarm-service:{SwarmServiceId}";
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(FileSystemBackupRepositorySpec), "FileSystem")]
[JsonDerivedType(typeof(S3CompatibleBackupRepositorySpec), "S3Compatible")]
public abstract record BackupRepositorySpec
{
    public abstract BackupRepositoryType Type { get; }
}

public sealed record FileSystemBackupRepositorySpec(
    BackupExecutionLocation Location,
    Guid? PlatformId,
    string Path) : BackupRepositorySpec
{
    public override BackupRepositoryType Type => BackupRepositoryType.FileSystem;
}

public sealed record S3CompatibleBackupRepositorySpec(
    Uri Endpoint,
    string Bucket,
    string? Prefix,
    string? Region,
    S3BucketLookup BucketLookup,
    Guid AccessKeySecretId,
    Guid SecretKeySecretId,
    Guid? SessionTokenSecretId,
    bool AllowInsecureHttp = false) : BackupRepositorySpec
{
    public override BackupRepositoryType Type => BackupRepositoryType.S3Compatible;
}

public sealed record BackupExecutionContext(BackupExecutionLocation Location, Guid? PlatformId)
{
    public void Validate()
    {
        if (Location == BackupExecutionLocation.Core && PlatformId.HasValue)
            throw new ArgumentException("Core backup execution cannot include a platform ID.", nameof(PlatformId));

        if (Location == BackupExecutionLocation.Platform && (!PlatformId.HasValue || PlatformId.Value == Guid.Empty))
            throw new ArgumentException("Platform backup execution requires a platform ID.", nameof(PlatformId));
    }
}
