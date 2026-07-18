namespace Infrastructure.Persistence.Dtos;

internal sealed record CurrentProfileDto(
    Guid Id,
    string DisplayName,
    string Email,
    Guid ActorId,
    DateTime CreatedAt,
    string Teams,
    string Roles,
    bool HasLocalPassword,
    Guid? OidcProviderId,
    string? OidcProviderName);
