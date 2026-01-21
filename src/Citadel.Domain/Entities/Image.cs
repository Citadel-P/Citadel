namespace Domain.Entities;

public sealed class Image(
    string name,
    IEnumerable<string> tags,
    string dockerImageId,
    double size,
    int containers,
    Guid platformId,
    DateTime createdAt,
    DateTime? updatedAt = null,
    Guid? registryId = null,
    Registry? registry = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public Guid? RegistryId { get; private set; } = registryId;
    public string DockerImageId { get; private set; } = dockerImageId;
    public int Containers { get; private set; } = containers;
    public IEnumerable<string> Tags { get; private set; } = [.. tags];
    public double Size { get; private set; } = size;
    public string Name { get; private set; } = name;
    public DateTime CreatedAt { get; private set; } = createdAt;
    public DateTime? UpdatedAt { get; private set; } = updatedAt;
    public Registry? Registry { get; private set; } = registry;

    public void PartialUpdate(
        string? name = null,
        string? dockerImageId = null,
        IEnumerable<string>? tags = null,
        double? size = null,
        int? containers = null,
        DateTime? updatedAt = null,
        Guid? registryId = null)
    {
        if (name is not null && Name != name)
            Name = name;
        if (tags is not null && !(new HashSet<string>(Tags, StringComparer.OrdinalIgnoreCase).SetEquals(tags)))
            Tags = tags;
        if (dockerImageId is not null && DockerImageId != dockerImageId)
            DockerImageId = dockerImageId;
        if (size is not null && Size != size)
            Size = size.Value;
        if (containers is not null && Containers != containers)
            Containers = containers.Value;
        if (updatedAt is not null && UpdatedAt != updatedAt)
            UpdatedAt = updatedAt.Value;
        if (registryId is not null && RegistryId != registryId)
            RegistryId = registryId;
    }

    public static Image FromPersistence(
        Guid id,
        string name,
        IEnumerable<string> tags,
        string dockerImageId,
        double size,
        int containers,
        Guid platformId,
        DateTime createdAt,
        DateTime? updatedAt = null,
        Guid? registryId = null,
        Registry? registry = null)
    {
        return new Image(
            name: name,
            tags: tags,
            dockerImageId: dockerImageId,
            size: size,
            containers: containers,
            platformId: platformId,
            createdAt: createdAt,
            updatedAt: updatedAt,
            registryId: registryId)
        { 
            Id = id,
            Registry = registry
        };
    }
}
