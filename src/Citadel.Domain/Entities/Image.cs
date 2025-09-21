namespace Domain.Entities;

public sealed class Image(
    string name,
    string tag,
    string imageId,
    double size,
    int containers,
    Guid platformId,
    DateTime createdAt,
    bool? isUpToDate = null,
    DateTime? updatedAt = null,
    Guid? registryId = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid PlatformId { get; private set; } = platformId;
    public Guid? RegistryId { get; private set; } = registryId;
    public string ImageId { get; private set; } = imageId;
    public int Containers { get; private set; } = containers;
    public bool? IsUpToDate { get; private set; } = isUpToDate;
    public string Tag { get; private set; } = tag;
    public double Size { get; private set; } = size;
    public string Name { get; private set; } = name;
    public DateTime CreatedAt { get; private set; } = createdAt;
    public DateTime? UpdatedAt { get; private set; } = updatedAt;
    public Registry? Registry { get; private set; }

    public void PartialUpdate(
        string? name = null,
        string? tag = null,
        string? imageId = null,
        double? size = null,
        int? containers = null,
        bool? isUpToDate = null,
        DateTime? updatedAt = null,
        Guid? registryId = null)
    {
        if (name is not null && Name != name)
            Name = name;
        if (tag is not null && Tag != tag)
            Tag = tag;
        if (imageId is not null && ImageId != imageId)
            ImageId = imageId;
        if (size is not null && Size != size)
            Size = size.Value;
        if (containers is not null && Containers != containers)
            Containers = containers.Value;
        if (isUpToDate is not null && IsUpToDate != isUpToDate)
            IsUpToDate = isUpToDate.Value;
        if (updatedAt is not null && UpdatedAt != updatedAt)
            UpdatedAt = updatedAt.Value;
        if (registryId is not null && RegistryId != registryId)
            RegistryId = registryId;
    }

    public static Image FromPersistence(
        Guid id,
        string name,
        string tag,
        string imageId,
        double size,
        int containers,
        Guid platformId,
        DateTime createdAt,
        bool? isUpToDate = null,
        DateTime? updatedAt = null,
        Guid? registryId = null,
        Registry? registry = null)
    {
        return new Image(
            name: name,
            tag: tag,
            imageId: imageId,
            size: size,
            containers: containers,
            platformId: platformId,
            createdAt: createdAt,
            isUpToDate: isUpToDate,
            updatedAt: updatedAt,
            registryId: registryId)
        { 
            Id = id,
            Registry = registry
        };
    }
}
