namespace Infrastructure.Persistence.Dtos;

internal sealed record StackReleaseDto(
    Guid Id,
    Guid StackId,
    Guid PlatformId,
    string Status,
    string Version,
    string Spec,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string? Platform_Name = null,
    string? Platform_Status = null
    );