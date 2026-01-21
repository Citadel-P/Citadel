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

    public Platform? Platform { get; private set; } = null;
    public Image? Image { get; private set; } = null;
    public Container? Container { get; private set; } = null;

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
        DeploymentStatus status,
        DateTime createdAt,
        Guid createdByActorId,
        UpdateBehavior updateBehavior,
        string? description = null,
        AutoUpdateState? autoUpdateState = null,
        DeploymentSpec? spec = null,
        Platform? platform = null,
        Image? image = null,
        Container? container = null)
    {
        return new Deployment(name, status, createdByActorId, platformId, updateBehavior, spec, description)
        {
            Id = id,
            CreatedAt = createdAt,
            AutoUpdateState = autoUpdateState,
            Platform = platform,
            Image = image,
            Container = container
        };
    }

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        Guid? platformId = null,
        DeploymentStatus? status = null,
        UpdateBehavior? updateBehavior = null,
        DeploymentSpec? spec = null,
        Container? container = null)
    {
        if (name != null) Name = name;
        if (status != null) Status = status.Value;
        if (description != null) Description = description;
        if (platformId != null) PlatformId = platformId.Value;
        if (spec != null) Spec = spec;
        if (updateBehavior != null) UpdateBehavior = updateBehavior.Value;
        if (container != null) Container = container;
    }

    public static DeploymentStatus ToDeploymentStatus(ContainerStateStatus status)
    {
        return status switch
        {
            ContainerStateStatus.Running => DeploymentStatus.Healthy,
            ContainerStateStatus.Restarting => DeploymentStatus.Pending,
            ContainerStateStatus.Paused => DeploymentStatus.Pending,
            ContainerStateStatus.Exited => DeploymentStatus.Stopped,
            ContainerStateStatus.Created => DeploymentStatus.Created,
            ContainerStateStatus.Offline => DeploymentStatus.Degraded,
            ContainerStateStatus.Dead => DeploymentStatus.Degraded,
            _ => DeploymentStatus.Failed,
        };
    }
}

public sealed record AutoUpdateState(
    DateTime LastCheckedAt,
    AutoUpdateStatus Status,
    string? CurrentDigest = null,
    string? RemoteDigest = null,
    string? LastError = null
    );