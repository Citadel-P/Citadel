using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public sealed record DeploymentSpec(
    DeploymentImageInfo Image,
    string? WorkingDir,
    string? User, 
    LifeCycleSpec? LifeCycleSpec,
    ResourceSpec? ResourceSpec,
    Dictionary<string, string>? Labels,
    List<string>? Ports,
    List<string>? EnvVars,
    List<string>? Volumes,
    List<string>? Networks,
    List<string>? EntryPoint,
    List<string>? Command,
    SecurityConfig? Security = null,
    LoggingConfig? LoggingConfig = null,
    HealthCheckConfig? HealthCheck = null
    );

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(LocalImage), nameof(ImageSource.Local))]
[JsonDerivedType(typeof(ExternalImage), nameof(ImageSource.External))]
public abstract record DeploymentImageInfo;

public sealed record LocalImage(string ImageId) : DeploymentImageInfo;
public sealed record ExternalImage(string RegistryId, string ImageTag) : DeploymentImageInfo;

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