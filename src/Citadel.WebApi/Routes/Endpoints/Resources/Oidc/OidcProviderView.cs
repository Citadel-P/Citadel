using Application.Features.Oidc.Commands;
using Application.Features.Oidc.Models;
using Application.Services;
using Domain.Entities.Oidc;

namespace WebApi.Routes.Endpoints.Resources.Oidc;

public sealed record OidcProviderView(
    Guid Id,
    string Name,
    string? Description,
    string DisplayName,
    string Issuer,
    string ClientId,
    string Scopes,
    bool Enabled,
    bool AutoProvisionUsers,
    bool AllowEmailAutoLink,
    bool RequireEmailVerified,
    string? AllowedEmailDomains,
    string? RequiredClaimName,
    string? RequiredClaimValues,
    Guid? DefaultRoleId,
    bool HasClientSecret,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt)
{
    internal static OidcProviderView Map(OidcProvider provider)
        => new(
            provider.Id,
            provider.Name,
            provider.Description,
            provider.DisplayName,
            provider.Issuer,
            provider.ClientId,
            provider.Scopes,
            provider.Enabled,
            provider.AutoProvisionUsers,
            provider.AllowEmailAutoLink,
            provider.RequireEmailVerified,
            provider.AllowedEmailDomains,
            provider.RequiredClaimName,
            provider.RequiredClaimValues,
            provider.DefaultRoleId,
            !string.IsNullOrWhiteSpace(provider.ClientSecretCiphertext),
            provider.CreatedByActorId,
            provider.CreatedAt,
            provider.UpdatedAt);
}

public sealed record OidcProvidersView(IReadOnlyList<OidcProviderView> Providers)
{
    internal static OidcProvidersView Map(OidcProviderListResult result)
        => new([.. result.Providers.Select(OidcProviderView.Map)]);
}

public sealed record OidcLoginProviderView(Guid Id, string DisplayName);

public sealed record OidcLoginProvidersView(IReadOnlyList<OidcLoginProviderView> Providers)
{
    internal static OidcLoginProvidersView Map(OidcProviderListResult result)
        => new([.. result.Providers.Select(static provider => new OidcLoginProviderView(provider.Id, provider.DisplayName))]);
}

public sealed record OidcProviderInput(
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
    Guid? DefaultRoleId)
{
    internal OidcProviderInputModel ToModel()
        => new(
            Name,
            Description,
            DisplayName,
            Issuer,
            ClientId,
            ClientSecret,
            Scopes,
            Enabled,
            AutoProvisionUsers,
            AllowEmailAutoLink,
            RequireEmailVerified,
            AllowedEmailDomains,
            RequiredClaimName,
            RequiredClaimValues,
            DefaultRoleId);
}

public sealed record UpdateOidcProviderInput(
    string? Name = null,
    string? Description = null,
    string? DisplayName = null,
    string? Issuer = null,
    string? ClientId = null,
    string? ClientSecret = null,
    string? Scopes = null,
    bool? Enabled = null,
    bool? AutoProvisionUsers = null,
    bool? AllowEmailAutoLink = null,
    bool? RequireEmailVerified = null,
    string? AllowedEmailDomains = null,
    string? RequiredClaimName = null,
    string? RequiredClaimValues = null,
    Guid? DefaultRoleId = null)
{
    internal UpdateOidcProviderInputModel ToModel()
        => new(
            Name,
            Description,
            DisplayName,
            Issuer,
            ClientId,
            ClientSecret,
            Scopes,
            Enabled,
            AutoProvisionUsers,
            AllowEmailAutoLink,
            RequireEmailVerified,
            AllowedEmailDomains,
            RequiredClaimName,
            RequiredClaimValues,
            DefaultRoleId);
}

public sealed record TestOidcProviderDiscoveryInput(Guid? ProviderId, string? Issuer);

public sealed record OidcDiscoveryResultView(
    string Issuer,
    string AuthorizationEndpoint,
    string TokenEndpoint,
    string JwksUri)
{
    internal static OidcDiscoveryResultView Map(OidcDiscoveryResult result)
        => new(result.Issuer, result.AuthorizationEndpoint, result.TokenEndpoint, result.JwksUri);
}
