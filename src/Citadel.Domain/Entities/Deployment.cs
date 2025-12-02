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
    public DateTime? UpdatedAt { get; private set; }
    public Guid CreatedBy { get; private set; } = createdBy;
    public Guid? UpdatedBy { get; private set; }

    public IReadOnlyCollection<DeploymentVersion> Versions => versions;
    public Guid? ActiveVersionId { get; private set; }

    public DeploymentVersion CreateVersion(DeploymentSpec spec, Guid platformId, Guid createdBy)
    {
        int nextVersion = versions.Count == 0
            ? 1
            : versions.Max(v => v.Version) + 1;

        var newVersion = new DeploymentVersion
        (
            deploymentId: Id,
            version: nextVersion,
            platformId: platformId,
            createdBy: createdBy,
            spec: spec,
            source: DeploymentSource.UI,
            status: DeploymentStatus.Created
        );
        versions.Add(newVersion);
        UpdatedAt = DateTime.UtcNow;
        return newVersion;
    }

    public void SetActiveVersion(Guid versionId, Guid updatedBy)
    {
        if (versions.All(v => v.Id != versionId))
            throw new ArgumentException("Version not found.", nameof(versionId));

        ActiveVersionId = versionId;
        UpdatedAt = DateTime.UtcNow;
        UpdatedBy = updatedBy;
    }
}