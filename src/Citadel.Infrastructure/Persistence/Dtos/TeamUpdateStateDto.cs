namespace Infrastructure.Persistence.Dtos;

internal sealed record TeamUpdateStateDto(
    Guid? Id,
    string? Name,
    Guid? ActorId,
    bool? IsEnabled,
    bool NameExists);
