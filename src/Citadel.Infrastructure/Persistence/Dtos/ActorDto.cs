namespace Infrastructure.Persistence.Dtos;

internal sealed record class ActorDto(
    Guid Id,
    string Type,
    string Name
    );
