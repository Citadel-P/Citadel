namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamWithActorDto(
    Guid Id,
    string Name,
    Guid ActorId,
    bool IsEnabled,
    int TotalMembers,
    string Roles);
