namespace Domain.Entities;

public sealed class Deployment(
    string name,
    DeploymentStatus status,
    Guid createdByActorId,
    Guid platformId,
    DeploymentSpec spec,
    string? description = null) : AuditedEntity(createdByActorId)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public DeploymentStatus Status { get; private set; } = status;

    public DeploymentSpec Spec { get; private set; } = spec;

    public void MarkAsDeployed()
    {
        Status = DeploymentStatus.Healthy;
    }

    public void MarkAsFailed()
    {
        Status = DeploymentStatus.Failed;
    }

    public static Deployment FromPersistence(
        Guid id,
        string name,
        Guid platformId,
        string? description,
        DeploymentStatus status,
        DateTime createdAt,
        Guid createdByActorId,
        DeploymentSpec spec)
    {
        return new Deployment(name, status, createdByActorId, platformId, spec, description)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        Guid? platformId = null,
        DeploymentStatus? status = null,
        DeploymentSpec? spec = null)
    {
        if (name != null) Name = name;
        if (status != null) Status = status.Value;
        if (description != null) Description = description;
        if (platformId != null) PlatformId = platformId.Value;
        if (spec != null) Spec = spec;
    }
}