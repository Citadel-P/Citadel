namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamDto(
    Guid Id,
    string Name,
    Guid ActorId);
