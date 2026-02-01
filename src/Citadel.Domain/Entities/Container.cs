namespace Domain.Entities;

public class Container(
    string name,
    string dockerImageId,
    Guid platformId,
    string dockerContainerId,
    ContainerStateStatus state,
    long? created = null,
    string? stack = null,
    Guid? deploymentId = null,
    Guid? imageId = null,
    IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null) : IReconcilableResource
{
    private readonly List<ContainerStat> stats = [];
    private readonly IDictionary<string, IReadOnlyList<HostPortBinding>> ports = ports is not null 
        ? ports : new Dictionary<string, IReadOnlyList<HostPortBinding>>();

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public Guid? DeploymentId { get; private set; } = deploymentId;
    public Guid? ImageId { get; private set; } = imageId;
    public string DockerContainerId { get; private set; } = dockerContainerId;
    public string? DockerImageId { get; private set; } = dockerImageId;
    public string Name { get; private set; } = name;
    public long Created { get; private set; } = created is not null ? created.Value : (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
    public long Updated { get; private set; }
    public ContainerStateStatus State { get; set; } = state;
    public string? Stack { get; private set; } = stack;

    #region IReconcilableResource Members
    public ResourceControlState ControlState { get; private set; } = ResourceControlState.Idle;
    public Guid? ControlTriggeredBy { get; private set; }
    public long? ControlStartedAt { get; private set; }
    public long RowVersion { get; private set; }
    #endregion

    public IDictionary<string, IReadOnlyList<HostPortBinding>> Ports => ports;
    public IReadOnlyCollection<ContainerStat>? Stats => stats;
    public Platform? Platform { get; private set; } = null!;
    public Image? Image { get; private set; } = null!;
    public Deployment? Deployment { get; private set; } = null!;

    public Container PartialUpdate(
        string? name = null,
        string ? dockerImageId = null,
        ContainerStateStatus? state = null,
        string? stack = null,
        long? created = null,
        Guid? platformId = null,
        Guid? imageId = null,
        Guid? deploymentId = null,
        IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null)
    {
        if (name != null) Name = name;
        if (dockerImageId != null) DockerImageId = dockerImageId;
        if (state != null) State = state.Value;
        if (stack != null) Stack = stack;
        if (created != null) Created = created.Value;
        if (platformId != null) PlatformId = platformId.Value;
        if (imageId is not null) ImageId = imageId;
        if (deploymentId is not null) DeploymentId = deploymentId;
        if (ports != null)
        {
            this.ports.Clear();
            foreach (var kvp in ports)
                this.ports[kvp.Key] = kvp.Value;
        }
        Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        return this;
    }

    public Container AppendStat(ContainerStat stat)
    {
        stats.Add(stat);
        return this;
    }

    public void MarkProcessing(Guid controlTriggeredBy)
    {
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }

    public void ReleaseProcessing()
    {
        ControlState = ResourceControlState.Idle;
        ControlTriggeredBy = null;
        ControlStartedAt = null;
    }

    public static Container FromPersistence (
        Guid id,
        Guid platformId,
        string dockerContainerId,
        string dockerImageId,
        string name,
        long created,
        long updated,
        long rowVersion,
        long? controlStartedAt,
        Guid? controlTriggeredBy,
        ResourceControlState controlState,
        ContainerStateStatus state,
        IDictionary<string, IReadOnlyList<HostPortBinding>> ports,
        string? stack = null,
        Guid? imageId = null,
        Guid? deploymentId = null,
        Image? image = null,
        Deployment? deployment = null,
        IReadOnlyCollection<ContainerStat>? stats = null
        )
    {
        var container = new Container(
            name: name,
            dockerContainerId: dockerContainerId,
            dockerImageId: dockerImageId,
            platformId: platformId,
            state: state,
            created: created,
            stack: stack,
            ports: ports,
            imageId: imageId,
            deploymentId: deploymentId)
        {
            Id = id,
            Image = image,
            Updated = updated,
            Deployment = deployment,
            RowVersion = rowVersion,
            ControlState = controlState,
            ControlStartedAt = controlStartedAt,
            ControlTriggeredBy = controlTriggeredBy
        };

        if (stats is not null)
        {
            foreach (var stat in stats)
                container.AppendStat(stat);
        }

        return container;
    }
}

public record struct HostPortBinding(string? HostIP, string? HostPort);