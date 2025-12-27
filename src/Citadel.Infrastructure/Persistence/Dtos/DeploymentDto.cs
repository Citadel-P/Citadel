namespace Infrastructure.Persistence.Dtos;

internal sealed record DeploymentDto(
    Guid Id,
    string Name,
    string Status,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    Guid PlatformId,
    string Spec,
    string? Description = null);
