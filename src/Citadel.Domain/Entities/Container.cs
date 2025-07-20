using System.ComponentModel;

namespace Domain.Entities;

public class Container(
    string name,
    string image,
    Guid platformId,
    string containerId,
    ContainerStateStatus state,
    long? created = null,
    string? stack = null,
    IEnumerable<ContainerPort>? ports = null)
{
    private readonly List<ContainerStat> stats = [];
    private readonly List<ContainerPort> ports = ports is not null ? [.. ports] : [];

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public string ContainerId { get; private set; } = containerId;
    public string Name { get; private set; } = name;
    public string Image { get; private set; } = image;
    public long Created { get; private set; } = created is not null ? created.Value : DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    public long Updated { get; private set; } = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    public ContainerStateStatus State { get; set; } = state;
    public string? Stack { get; set; } = stack;
    public IReadOnlyCollection<ContainerPort> Ports => ports;
    public IReadOnlyCollection<ContainerStat>? Stats => stats;
    public Platform? Platform { get; private set; } = null!;

    public Container PartialUpdate(
        string? name = null,
        string? image = null,
        ContainerStateStatus? state = null,
        string? stack = null,
        long? created = null,
        Guid? platformId = null,
        IEnumerable<ContainerPort>? ports = null)
    {
        if (name != null) Name = name;
        if (image != null) Image = image;
        if (state != null) State = state.Value;
        if (stack != null) Stack = stack;
        if (created != null) Created = created.Value;
        if (platformId != null) PlatformId = platformId.Value;
        if (ports != null)
        {
            this.ports.Clear();
            this.ports.AddRange(ports);
        }
        Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        return this;
    }

    public Container AppendStat(ContainerStat stat)
    {
        if (stat is null) throw new ArgumentNullException(nameof(stat), "Stat cannot be null.");
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
        long created,
        long updated,
        ContainerStateStatus state,
        IReadOnlyCollection<ContainerPort> ports,
        string? stack = null,
        Platform? platform = null,
        IReadOnlyCollection<ContainerStat>? stats = null
        )
    {
        var container = new Container(
            name: name,
            image: image,
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

public record struct ContainerPort(
    string IP, 
    int? PrivatePort, 
    int? PublicPort,
    string? Type = null);