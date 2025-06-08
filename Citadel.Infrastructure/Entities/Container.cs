namespace Infrastructure.Entities;

public class Container
{
    private readonly List<ContainerStat> stats = [];
    private readonly List<ContainerPort> ports = [];

    private Container() { /* For EF Core */ }

    public Container(
        Guid platformId,
        string containerId,
        string name,
        string image,
        ContainerStateStatus state,
        string stack,
        long? created = null,
        IEnumerable<ContainerPort>? ports = null)
    {
        Id = Guid.CreateVersion7();
        PlatformId = platformId;
        ContainerId = containerId;
        Name = name;
        Image = image;
        State = state;
        Stack = stack;
        Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        Created = created is not null ? created.Value : DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        this.ports = ports is not null ? [.. ports] : [];
    }

    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string ContainerId { get; private set; } = null!;
    public string Name { get; private set; } = null!;
    public string Image { get; private set; } = null!;
    public long Created { get; private set; }
    public long Updated { get; private set; }
    public ContainerStateStatus State { get; set; }
    public string? Stack { get; set; }
    public IReadOnlyCollection<ContainerPort> Ports => ports;
    public IReadOnlyCollection<ContainerStat> Stats => stats;
    public Platform Platform { get; private set; } = null!;

    public void PartialUpdate(
        string? name = null,
        string? image = null,
        ContainerStateStatus? state = null,
        string? stack = null,
        long? created = null,
        IEnumerable<ContainerPort>? ports = null)
    {
        if (name != null) Name = name;
        if (image != null) Image = image;
        if (state != null) State = state.Value;
        if (stack != null) Stack = stack;
        if (created != null) Created = created.Value;
        if (ports != null)
        {
            this.ports.Clear();
            this.ports.AddRange(ports);
        }
        Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }

    public Container AppendStat(ContainerStat stat)
    {
        if (stat is null) throw new ArgumentNullException(nameof(stat), "Stat cannot be null.");
        stats.Add(stat);
        return this;
    }
}

public record struct ContainerPort(
    string IP, 
    int? PrivatePort, 
    int? PublicPort);