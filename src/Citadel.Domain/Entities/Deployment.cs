namespace Domain.Entities;

public sealed class Deployment(
    string name,
    DeploymentStatus status,
    Guid createdByActorId,
    Guid platformId,
    UpdateBehavior updateBehavior,
    DeploymentSpec? spec = null,
    string? description = null) : AuditedEntity(createdByActorId)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;

    public DeploymentStatus Status { get; private set; } = status;

    public UpdateBehavior UpdateBehavior { get; private set; } = updateBehavior;
    public AutoUpdateState? AutoUpdateState { get; private set; } = new AutoUpdateState(LastCheckedAt: DateTime.MinValue, Status: AutoUpdateStatus.Unknown);

    public DeploymentSpec? Spec { get; private set; } = spec;

    public Platform? Platform { get; private set; } = null!;
    public Image? Image { get; private set; } = null!;

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
        UpdateBehavior updateBehavior,
        AutoUpdateState? autoUpdateState,
        DeploymentSpec? spec,
        Platform? platform = null,
        Image? image = null)
    {
        return new Deployment(name, status, createdByActorId, platformId, updateBehavior, spec, description)
        {
            Id = id,
            CreatedAt = createdAt,
            AutoUpdateState = autoUpdateState,
            Platform = platform,
            Image = image
        };
    }

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        Guid? platformId = null,
        DeploymentStatus? status = null,
        UpdateBehavior? updateBehavior = null,
        DeploymentSpec? spec = null)
    {
        if (name != null) Name = name;
        if (status != null) Status = status.Value;
        if (description != null) Description = description;
        if (platformId != null) PlatformId = platformId.Value;
        if (spec != null) Spec = spec;
        if (updateBehavior != null) UpdateBehavior = updateBehavior.Value;
    }
}

public sealed record AutoUpdateState(
    DateTime LastCheckedAt,
    AutoUpdateStatus Status,
    string? CurrentDigest = null,
    string? RemoteDigest = null,
    string? LastError = null
    );