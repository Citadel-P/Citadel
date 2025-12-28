using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public sealed record DeploymentSpec(
    DeploymentImageInfo Image,
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
public sealed record ExternalImage(Guid RegistryId, string ImageTag) : DeploymentImageInfo;

public sealed record ResourceSpec(
    float? CpuLimit,
    float? MemoryLimit,
    float? MemoryReservation
);

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