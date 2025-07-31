namespace Infrastructure.Persistence.Dtos;

internal sealed record ContainerInfoDto(
    Guid Id,
    string Name,
    string ContainerId,
    Guid PlatformId,
    string PlatformName,
    string State);
