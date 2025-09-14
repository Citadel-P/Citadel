namespace Domain.Entities;

public class Container(
    string name,
    string image,
    string imageId,
    Guid platformId,
    string containerId,
    ContainerStateStatus state,
    long? created = null,
    string? stack = null,
    Guid? deploymentId = null,
    IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null)
{
    private readonly List<ContainerStat> stats = [];
    private readonly IDictionary<string, IReadOnlyList<HostPortBinding>> ports = ports is not null 
        ? ports : new Dictionary<string, IReadOnlyList<HostPortBinding>>();

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public Guid? DeploymentId { get; private set; } = deploymentId;
    public string ContainerId { get; private set; } = containerId;
    public string Name { get; private set; } = name;
    public string Image { get; private set; } = image;
    public long Created { get; private set; } = created is not null ? created.Value : (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
    public long Updated { get; private set; } = (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds;
    public ContainerStateStatus State { get; set; } = state;
    public string? Stack { get; private set; } = stack;
    public string? ImageId { get; private set; } = imageId;
    public IDictionary<string, IReadOnlyList<HostPortBinding>> Ports => ports;
    public IReadOnlyCollection<ContainerStat>? Stats => stats;
    public Platform? Platform { get; private set; } = null!;

    public Container PartialUpdate(
        string? name = null,
        string? image = null,
        string ? imageId = null,
        ContainerStateStatus? state = null,
        string? stack = null,
        long? created = null,
        Guid? platformId = null,
        IDictionary<string, IReadOnlyList<HostPortBinding>>? ports = null)
    {
        if (name != null) Name = name;
        if (image != null) Image = image;
        if (imageId != null) ImageId = imageId;
        if (state != null) State = state.Value;
        if (stack != null) Stack = stack;
        if (created != null) Created = created.Value;
        if (platformId != null) PlatformId = platformId.Value;
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

    public static Container FromPersistence
        (
        Guid id,
        Guid platformId,
        string containerId,
        string name,
        string image,
        string imageId,
        long created,
        long updated,
        ContainerStateStatus state,
        IDictionary<string, IReadOnlyList<HostPortBinding>> ports,
        string? stack = null,
        Platform? platform = null,
        IReadOnlyCollection<ContainerStat>? stats = null
        )
    {
        var container = new Container(
            name: name,
            image: image,
            imageId: imageId,
            platformId: platformId,
            containerId: containerId,
            state: state,
            created: created,
            stack: stack,
            ports: ports)
        {
            Id = id,
            Updated = updated,
            Platform = platform,
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