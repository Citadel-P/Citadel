namespace Infrastructure.Persistence.Dtos;

internal sealed record UserUpdateStateDto(
    Guid? Id,
    string? Name,
    string? Email,
    string? Password,
    Guid? ActorId,
    bool? IsEnabled,
    DateTime? CreatedAt,
    Guid? CreatedByActorId,
    bool NameExists,
    bool EmailExists);
