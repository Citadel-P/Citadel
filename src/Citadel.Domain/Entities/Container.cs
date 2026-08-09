using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;

namespace Domain.Entities;

public class Container(
    string name,
    string dockerImageId,
    Guid platformId,
    string dockerContainerId,
    ContainerStateStatus state,
    long? created = null,
    string? dockerStack = null,
    Guid? deploymentId = null,
    Guid? stackId = null,
    Guid? imageId = null,
    IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null,
    bool isSystem = false,
    ContainerSystemRole? systemRole = null,
    bool hasCitadelOwnershipLabels = false,
    bool isSwarmTask = false) : IReconcilableResource
{
    private readonly List<ContainerStat> stats = [];
    private readonly IDictionary<string, IReadOnlyList<HostPortBinding>> ports = ports is not null 
        ? ports : new Dictionary<string, IReadOnlyList<HostPortBinding>>();

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public Guid? DeploymentId { get; private set; } = deploymentId;
    public Guid? StackId { get; private set; } = stackId;
    public Guid? ImageId { get; private set; } = imageId;
    public string DockerContainerId { get; private set; } = dockerContainerId;
    public string? DockerImageId { get; private set; } = dockerImageId;
    public string Name { get; private set; } = name;
    public long Created { get; private set; } = created is not null ? created.Value : (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
    public long Updated { get; private set; }
    public ContainerStateStatus State { get; set; } = state;
    public string? DockerStack { get; private set; } = dockerStack;
    public bool IsSystem { get; private set; } = isSystem;
    public ContainerSystemRole? SystemRole { get; private set; } = isSystem ? systemRole : null;
    public bool HasCitadelOwnershipLabels { get; private set; } = hasCitadelOwnershipLabels;
    public bool IsSwarmTask { get; private set; } = isSwarmTask;

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
    public Stack? Stack { get; private set; } = null!;

    public Container PartialUpdate(
        string? name = null,
        string ? dockerImageId = null,
        ContainerStateStatus? state = null,
        string? dockerStack = null,
        long? created = null,
        Guid? platformId = null,
        Guid? imageId = null,
        Guid? deploymentId = null,
        Guid? stackId = null,
        IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null,
        bool? isSystem = null,
        ContainerSystemRole? systemRole = null,
        bool? hasCitadelOwnershipLabels = null,
        bool? isSwarmTask = null)
    {
        if (name != null) Name = name;
        if (dockerImageId != null) DockerImageId = dockerImageId;
        if (state != null) State = state.Value;
        if (dockerStack != null) DockerStack = dockerStack;
        if (created != null) Created = created.Value;
        if (platformId != null) PlatformId = platformId.Value;
        if (imageId is not null) ImageId = imageId;
        if (deploymentId is not null) DeploymentId = deploymentId;
        if (stackId is not null) StackId = stackId;
        if (isSystem is not null)
        {
            IsSystem = isSystem.Value;
            SystemRole = isSystem.Value ? systemRole : null;
        }
        if (hasCitadelOwnershipLabels is not null)
            HasCitadelOwnershipLabels = hasCitadelOwnershipLabels.Value;
        if (isSwarmTask is not null)
            IsSwarmTask = isSwarmTask.Value;
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
        string? dockerStack = null,
        Guid? imageId = null,
        Guid? deploymentId = null,
        Guid? stackId = null,
        Image? image = null,
        Stack? stack = null,
        Deployment? deployment = null,
        IReadOnlyCollection<ContainerStat>? stats = null,
        bool isSystem = false,
        ContainerSystemRole? systemRole = null,
        bool hasCitadelOwnershipLabels = false,
        bool isSwarmTask = false
        )
    {
        var container = new Container(
            name: name,
            dockerContainerId: dockerContainerId,
            dockerImageId: dockerImageId,
            platformId: platformId,
            state: state,
            created: created,
            dockerStack: dockerStack,
            ports: ports,
            imageId: imageId,
            stackId: stackId,
            deploymentId: deploymentId,
            isSystem: isSystem,
            systemRole: systemRole,
            hasCitadelOwnershipLabels: hasCitadelOwnershipLabels,
            isSwarmTask: isSwarmTask)
        {
            Id = id,
            Image = image,
            Stack = stack,
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
