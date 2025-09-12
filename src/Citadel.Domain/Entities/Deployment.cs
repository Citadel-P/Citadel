namespace Domain.Entities;

public sealed class Deployment(
    Guid platformId,
    string containerName,
    string configJson,
    int version,
    DateTime created,
    DateTime updated)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public string ContainerName { get; private set; } = containerName;
    public string ConfigJson { get; private set; } = configJson;
    public int Version { get; private set; } = version;
    public DateTime Created { get; private set; } = created;
    public DateTime Updated { get; private set; } = updated;
}
