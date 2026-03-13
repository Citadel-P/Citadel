using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Deployments;

public sealed record DeploymentSpec(
    DeploymentImageInfo Image,
    UpdateBehavior UpdateBehavior,
    LifeCycleSpec? LifeCycleSpec = null,
    ResourceSpec? ResourceSpec = null,
    Dictionary<string, string>? Labels = null,
    List<string>? Ports = null,
    List<string>? EnvVars = null,
    List<string>? Volumes = null,
    List<string>? Networks = null,
    List<string>? Command = null
    );

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(LocalImage), nameof(ImageSource.Local))]
[JsonDerivedType(typeof(ExternalImage), nameof(ImageSource.External))]
public abstract record DeploymentImageInfo;

public sealed record LocalImage(string ImageId) : DeploymentImageInfo;
public sealed record ExternalImage(Guid RegistryId, string ImageTag, string? ResolvedDigest = null) : DeploymentImageInfo;
public sealed record ResourceSpec(
    float? NanoCpus,
    float? MemoryLimit
)
{
    public static readonly ResourceSpec Empty = new(null, null);
}

public sealed record LifeCycleSpec(
    int? StopTimeout,
    StopSignal? StopSignal,
    ContainerRestartPolicy RestartPolicy
    );

public sealed record HealthCheckConfig(
    IEnumerable<string> Test,
    string Interval = "30s",
    string Timeout = "5s",
    int Retries = 3,
    string StartPeriod = "0s"
    );

public sealed record LoggingConfig(
    LoggingDriverType Driver,
    Dictionary<string, string>? Options = null
    );

public sealed record SecurityConfig(
    bool Privileged = false,
    List<string>? CapAdd = null,
    List<string>? CapDrop = null,
    bool? ReadOnlyRootFs = null,
    List<string>? SecurityOpt = null
    );