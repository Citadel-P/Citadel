namespace Infrastructure.Persistence.Dtos;

internal sealed record UserWithActorDto(
    Guid Id,
    string Name,
    string Email,
    string Password,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId);
