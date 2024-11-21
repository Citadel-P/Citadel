namespace Infrastructure.Entities;

public class ContainerInfo
{
    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string ContainerId { get; private set; }
    public string Name { get; private set; }
    public string Image { get; private set; }
    public DateTimeOffset Created { get; private set; }
    public string State { get; set; }
    public string Status { get; private set; }
    public ICollection<ContainerPort> Ports { get; private set; } = [];
    public ICollection<ContainerStat> Stats { get; private set; } = [];
    public IDictionary<string, string> Labels { get; private set; } = new Dictionary<string, string>();
    public Platform Platform { get; private set; }

    public static ContainerInfo Create(
        Guid platformId, 
        string containerId,
        string name,
        string image,
        int created, 
        string state,
        string status,
        IEnumerable<ContainerPort> ports,
        IDictionary<string, string> labels = null)
        => new()
        {
            Id = Guid.CreateVersion7(),
            PlatformId = platformId,
            ContainerId = containerId,
            Name = name,
            Image = image,
            Created = DateTimeOffset.FromUnixTimeSeconds(created),
            State = state,
            Status = status,
            Ports = ports.ToList(),
            Labels = labels,
        };

    public void UpdateWith(
        string name,
        string image,
        int created,
        string state,
        string status,
        IEnumerable<ContainerPort> ports,
        IDictionary<string, string> labels = null)
    {
        Name = name;
        Image = image;
        Created = DateTimeOffset.FromUnixTimeSeconds(created);
        State = state;
        Status = status;
        Labels = labels;
        Ports = ports.ToList();
    }
}
