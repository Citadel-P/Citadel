using Domain.Entities.Oidc;

namespace Application.Features.Oidc.Models;

public sealed record OidcProviderInputModel(
    string Name,
    string? Description,
    string DisplayName,
    string Issuer,
    string ClientId,
    string? ClientSecret,
    string? Scopes,
    bool Enabled,
    bool AutoProvisionUsers,
    bool AllowEmailAutoLink,
    bool RequireEmailVerified,
    string? AllowedEmailDomains,
    string? RequiredClaimName,
    string? RequiredClaimValues,
    Guid? DefaultRoleId);

public sealed record UpdateOidcProviderInputModel(
    string? Name,
    string? Description,
    string? DisplayName,
    string? Issuer,
    string? ClientId,
    string? ClientSecret,
    string? Scopes,
    bool? Enabled,
    bool? AutoProvisionUsers,
    bool? AllowEmailAutoLink,
    bool? RequireEmailVerified,
    string? AllowedEmailDomains,
    string? RequiredClaimName,
    string? RequiredClaimValues,
    Guid? DefaultRoleId);

public sealed record OidcProviderListResult(IReadOnlyList<OidcProvider> Providers);
