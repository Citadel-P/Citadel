namespace Infrastructure.Persistence.Dtos;

internal sealed record ImageDto(
    Guid Id,
    string Name,
    string Tag,
    string ImageId,
    double Size,
    bool IsInUse,
    Guid PlatformId,
    DateTime CreatedAt,
    bool? IsUpToDate = null,
    DateTime? UpdatedAt = null,
    Guid? RegistryId = null
    );
