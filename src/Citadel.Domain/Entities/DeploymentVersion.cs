namespace Domain.Entities;

public sealed class DeploymentVersion (
    Guid deploymentId,
    int version,
    Guid platformId,
    DeploymentSpec spec,
    Guid createdBy,
    DeploymentSource source,
    DeploymentStatus status
    )
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid DeploymentId { get; private set; } = deploymentId;
    public int Version { get; private set; } = version;
    public Guid PlatformId { get; private set; } = platformId;
    public DeploymentSpec Spec { get; private set; } = spec;

    public DeploymentStatus Status { get; private set; } = status;
    public DeploymentSource Source { get; private set; } = source;

    public Guid CreatedBy { get; private set; } = createdBy;
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime? UpdatedAt { get; private set; }
    public Guid? UpdatedBy { get; private set; }

    public int? RolledBackFromVersion { get; private set; }

    // GitOps metadata
    public string? GitRepoUrl { get; private set; }
    public string? GitPath { get; private set; }
    public string? GitCommitHash { get; private set; }

    public Dictionary<string, string>? Annotations { get; private set; }

    public void MarkAsDeployed()
    {
        Status = DeploymentStatus.Healthy;
        UpdatedAt = DateTime.UtcNow;
    }

    public void MarkAsFailed()
    {
        Status = DeploymentStatus.Failed;
        UpdatedAt = DateTime.UtcNow;
    }

    public void MarkAsRolledBack(int rolledBackFrom)
    {
        RolledBackFromVersion = rolledBackFrom;
        Status = DeploymentStatus.RolledBack;
        UpdatedAt = DateTime.UtcNow;
    }
}