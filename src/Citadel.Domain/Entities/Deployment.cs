namespace Domain.Entities;

public sealed class Deployment(
    string name,
    Guid createdByActorId,
    Guid platformId,
    DeploymentSpec? spec = null,
    string? description = null) : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;

    public DeploymentStatus Status { get; private set; } = DeploymentStatus.Created;

    public AutoUpdateState? AutoUpdateState { get; private set; } = new AutoUpdateState(LastCheckedAt: DateTime.MinValue, Status: AutoUpdateStatus.Unknown);

    #region IReconcilableResource Members
    public ResourceControlState ControlState { get; private set; } = ResourceControlState.Idle;
    public Guid? ControlTriggeredBy { get; private set; }
    public long? ControlStartedAt { get; private set; }
    public long RowVersion { get; private set; }
    #endregion

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    public DeploymentSpec? Spec { get; private set; } = spec;

    public Platform? Platform { get; private set; } = null;
    public Image? Image { get; private set; } = null;
    public Container? Container { get; private set; } = null;

    public void MarkProcessing(Guid controlTriggeredBy)
    {
        Status = DeploymentStatus.Pending;
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }

    public void ReleaseProcessing(DeploymentStatus status)
    {
        Status = status;
        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
    }

    public static Deployment FromPersistence(
        Guid id,
        string name,
        Guid platformId,
        long rowVersion,
        long? controlStartedAt,
        ResourceControlState controlState,
        DeploymentStatus status,
        DateTime createdAt,
        Guid createdByActorId,
        Guid? controlTriggeredBy,
        string? description = null,
        AutoUpdateState? autoUpdateState = null,
        DeploymentSpec? spec = null,
        Platform? platform = null,
        Image? image = null,
        Container? container = null)
    {
        return new Deployment(name, createdByActorId, platformId, spec, description)
        {
            Id = id,
            Image = image,
            Status = status,
            Platform = platform,
            CreatedAt = createdAt,
            Container = container,
            RowVersion = rowVersion,
            ControlState = controlState,
            AutoUpdateState = autoUpdateState,
            ControlStartedAt = controlStartedAt,
            ControlTriggeredBy = controlTriggeredBy
        };
    }

    public void SetAutoUpdateState(AutoUpdateState? autoUpdateState)
    {
        AutoUpdateState = autoUpdateState;
    }

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        Guid? platformId = null,
        DeploymentStatus? status = null,
        ResourceControlState? resourceControlState = null,
        DeploymentSpec? spec = null,
        Container? container = null)
    {
        if (name != null) Name = name;
        if (status != null) Status = status.Value;
        if (description != null) Description = description;
        if (platformId != null) PlatformId = platformId.Value;
        if (spec != null) Spec = spec;
        if (container != null) Container = container;
        if (resourceControlState != null) ControlState = resourceControlState.Value;
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