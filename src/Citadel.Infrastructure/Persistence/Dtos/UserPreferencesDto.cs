namespace Infrastructure.Persistence.Dtos;

internal sealed record UserPreferencesDto(
    Guid UserId,
    string TimeZone,
    string DateTimeFormat,
    string Theme,
    DateTime UpdatedAt);
