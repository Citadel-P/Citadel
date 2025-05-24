namespace Infrastructure.Entities;

public class ContainerInfo
{
    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string ContainerId { get; private set; } = null!;
    public string Name { get; private set; } = null!;
    public string Image { get; private set; } = null!;
    public long Created { get; private set; }
    public long Updated { get; private set; }
    public ContainerStateStatus State { get; set; }
    public string? Stack { get; set; }
    public ICollection<ContainerPort> Ports { get; private set; } = [];
    public ICollection<ContainerStat> Stats { get; private set; } = [];
    public Platform Platform { get; private set; } = null!;

    public static ContainerInfo Create(
        Guid platformId, 
        string containerId,
        string name,
        string image,
        ContainerStateStatus state,
        string stack,
        long? created = null, 
        IEnumerable<ContainerPort>? ports = null)
        => new()
        {
            Id = Guid.CreateVersion7(),
            PlatformId = platformId,
            ContainerId = containerId,
            Name = name,
            Image = image,
            State = state,
            Stack = stack,
            Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            Created = created is not null ? created.Value : DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            Ports = ports is not null ? ports.ToList() : [],
        };

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
        if (ports != null) Ports = [.. ports];
        if (created != null) Created = created.Value;
        Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }
}
