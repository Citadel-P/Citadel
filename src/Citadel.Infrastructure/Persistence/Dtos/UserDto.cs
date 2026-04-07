namespace Infrastructure.Persistence.Dtos;

internal sealed record UserDto(
    Guid Id,
    string Name,
    string Email,
    string Password,
    Guid ActorId,
    DateTime CreatedAt,
    Guid CreatedByActorId);
