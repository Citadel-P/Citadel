namespace Infrastructure.Persistence.Dtos;

internal sealed record UserConflictCheckDto(bool NameExists, bool EmailExists);
