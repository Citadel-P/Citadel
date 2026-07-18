using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Profile;

public sealed record CurrentProfileView(
    Guid Id,
    string DisplayName,
    string Email,
    CurrentProfileAuthenticationView Authentication,
    DateTime CreatedAt,
    IReadOnlyCollection<ProfileResourceInfoView> DirectRoles,
    IReadOnlyCollection<ProfileResourceInfoView> Teams)
{
    internal static CurrentProfileView Map(CurrentProfileDetails profile)
    {
        var isOidc = profile.OidcProviderId.HasValue;
        return new CurrentProfileView(
            profile.Id,
            profile.DisplayName,
            profile.Email,
            new CurrentProfileAuthenticationView(
                isOidc ? CurrentProfileAuthenticationType.Oidc : CurrentProfileAuthenticationType.Local,
                isOidc ? $"Managed by {profile.OidcProviderName ?? "identity provider"}" : "Local account",
                !isOidc,
                profile.HasLocalPassword,
                profile.OidcProviderId,
                profile.OidcProviderName),
            profile.CreatedAt,
            [.. profile.DirectRoles.Select(ProfileResourceInfoView.Map)],
            [.. profile.Teams.Select(ProfileResourceInfoView.Map)]);
    }
}

public sealed record ProfileResourceInfoView(Guid Id, string Name)
{
    internal static ProfileResourceInfoView Map(ResourceInfo resource)
        => new(resource.Id, resource.Name);
}

public sealed record CurrentProfileAuthenticationView(
    CurrentProfileAuthenticationType Type,
    string Label,
    bool CanChangePassword,
    bool CanUseLocalPasswordMfa,
    Guid? OidcProviderId = null,
    string? OidcProviderName = null);

public enum CurrentProfileAuthenticationType
{
    Local,
    Oidc
}
