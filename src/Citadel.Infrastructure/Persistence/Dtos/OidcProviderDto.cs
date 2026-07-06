namespace Infrastructure.Persistence.Dtos;

internal sealed record OidcProviderDto(
    Guid Id,
    string Name,
    string? Description,
    string DisplayName,
    string Issuer,
    string ClientId,
    string? ClientSecretCiphertext,
    string Scopes,
    bool Enabled,
    bool AutoProvisionUsers,
    bool AllowEmailAutoLink,
    bool RequireEmailVerified,
    string? AllowedEmailDomains,
    string? RequiredClaimName,
    string? RequiredClaimValues,
    Guid? DefaultRoleId,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt);
