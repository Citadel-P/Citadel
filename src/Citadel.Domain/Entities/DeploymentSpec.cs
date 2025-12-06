using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities;

public sealed record DeploymentSpec(
    DeploymentImageInfo Image,
    string? Name,
    string? WorkingDir,
    string? User,
    float? MemoryLimit,
    float? CpuLimit,
    float? MemoryReservation,
    bool? AutoRemove,
    int? StopTimeout,
    ContainerRestartPolicy RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    List<string>? Networks,
    List<string>? EntryPoint,
    List<string>? Command,
    string? Hostname = null,
    List<string>? Dns = null,
    SecurityConfig? Security = null,
    LoggingConfig? LoggingConfig = null,
    HealthCheckConfig? HealthCheck = null,
    Dictionary<string, string>? Metadata = null
    );

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(LocalImage), nameof(ImageSource.Local))]
[JsonDerivedType(typeof(ExternalImage), nameof(ImageSource.External))]
public abstract record DeploymentImageInfo
{
    public abstract ImageSource Source { get; }
}

public sealed record LocalImage(string ImageId) : DeploymentImageInfo
{
    public override ImageSource Source => ImageSource.Local;
}
public sealed record ExternalImage(string RegistryId, string ImageName) : DeploymentImageInfo
{
    public override ImageSource Source => ImageSource.External;
}

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
    bool Privileged,
    List<string> CapAdd,
    List<string> CapDrop,
    bool ReadOnlyRootFs,
    List<string> SecurityOpt
    );