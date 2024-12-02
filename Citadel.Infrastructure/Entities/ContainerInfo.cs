namespace Infrastructure.Entities;

public class ContainerInfo
{
    public Guid Id { get; private set; }
    public Guid PlatformId { get; private set; }
    public string ContainerId { get; private set; }
    public string Name { get; private set; }
    public string Image { get; private set; }
    public int Created { get; private set; }
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
        string state,
        string status,
        int? created = null, 
        IEnumerable<ContainerPort> ports = null,
        IDictionary<string, string> labels = null)
        => new()
        {
            Id = Guid.CreateVersion7(),
            PlatformId = platformId,
            ContainerId = containerId,
            Name = name,
            Image = image,
            State = state,
            Status = status,
            Ports = ports?.ToList(),
            Labels = labels,
            Created = created.Value,
        };

    public void UpdateWith(
        string name = null,
        string image = null,
        string state = null,
        string status = null,
        int? created = null,
        IEnumerable<ContainerPort> ports = null,
        IDictionary<string, string> labels = null)
    {
        if (name != null) Name = name;
        if (image != null) Image = image;
        if (state != null) State = state;
        if (status != null) Status = status;
        if (labels != null) Labels = labels;
        if (labels != ports) Ports = ports.ToList();
        if (created != null) Created = created.Value;
    }
}
