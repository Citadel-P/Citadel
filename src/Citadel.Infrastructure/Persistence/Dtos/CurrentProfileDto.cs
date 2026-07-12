namespace Infrastructure.Persistence.Dtos;

internal sealed record CurrentProfileDto(
    Guid Id,
    string DisplayName,
    string Email,
    Guid ActorId,
    DateTime CreatedAt,
    string Teams,
    string Roles,
    Guid? OidcProviderId,
    string? OidcProviderName);
