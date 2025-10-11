namespace Domain.Entities;

public sealed class Deployment(
    string name,
    Guid createdBy,
    string? description)
{
    private readonly List<DeploymentVersion> versions = [];
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; private set; }
    public Guid CreatedBy { get; private set; } = createdBy;
    public IReadOnlyCollection<DeploymentVersion> Versions => versions.AsReadOnly();
    public int ActiveVersion { get; private set; }

    public DeploymentVersion CreateVersion(DeploymentSpec spec, Guid createdBy)
    {
        var newVersion = new DeploymentVersion
        (
            Version: versions.Count + 1,
            CreatedBy: createdBy,
            Spec: spec,
            Source: DeploymentSource.UI,
            Status: DeploymentStatus.Created
        );
        versions.Add(newVersion);
        UpdatedAt = DateTime.UtcNow;
        return newVersion;
    }

    public Deployment SetActiveVersion(int versionNumber)
    {
        if (versions.All(v => v.Version != versionNumber))
            throw new ArgumentException("Version not found.", nameof(versionNumber));
        
        ActiveVersion = versionNumber;
        return this;
    }
}

public sealed record DeploymentVersion (
    int Version,
    Guid CreatedBy,
    DeploymentSpec Spec,
    DeploymentStatus Status,
    DeploymentSource Source,
    int? RolledBackFromVersion = null,
    string? GitRepoUrl = null, 
    string? GitPath = null, 
    string? GitCommitHash = null,
    Dictionary<string, string>? Annotations = null
    )
{
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;

    public DeploymentVersion WithStatus(DeploymentStatus newStatus)
    => this with { Status = newStatus };
}

public sealed record DeploymentSpec(
    string ImageId,
    DeploymentTarget Target,
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
    DeploymentScaling? DeploymentScaling = null,
    SecurityConfig? Security = null,
    LoggingConfig? LoggingConfig = null,
    HealthCheckConfig? HealthCheck = null,
    Dictionary<string, string>? Metadata = null
    );

public sealed record DeploymentTarget(
    Guid PlatformId,
    string? PlatformName,
    int Replicas = 1
    );

public sealed record DeploymentScaling(
    ScalingStrategy Strategy, 
    int Replicas = 1,
    int MaxParallel = 1
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
    bool Privileged,
    List<string> CapAdd,
    List<string> CapDrop,
    bool ReadOnlyRootFs,
    List<string> SecurityOpt
    );