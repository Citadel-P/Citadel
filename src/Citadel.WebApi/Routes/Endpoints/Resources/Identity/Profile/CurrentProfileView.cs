using Application.Permissions;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Profile;

public sealed record CurrentProfileView(
    Guid Id,
    string DisplayName,
    string Email,
    CurrentProfileAuthenticationView Authentication,
    CurrentProfileAuthorizationView Authorization,
    DateTime CreatedAt,
    IReadOnlyCollection<ProfileResourceInfoView> DirectRoles,
    IReadOnlyCollection<ProfileResourceInfoView> Teams)
{
    internal static async Task<CurrentProfileView> Map(
        CurrentProfileDetails profile,
        IPermissionEvaluator permissionEvaluator)
    {
        var isOidc = profile.OidcProviderId.HasValue;
        var alertPermissions = await permissionEvaluator.EvaluateAsync(ResourceType.Alert);
        var alertChannelPermissions = await permissionEvaluator.EvaluateAsync(ResourceType.AlertChannel);
        var bindingPermissions = await permissionEvaluator.EvaluateAsync(ResourceType.Binding);
        var tagPermissions = await permissionEvaluator.EvaluateAsync(ResourceType.Tag);
        var alertCapabilities = CapabilityMapper.ToResourceCapabilities(alertPermissions);
        var alertChannelCapabilities = CapabilityMapper.ToResourceCapabilities(alertChannelPermissions);

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
            new CurrentProfileAuthorizationView(
                permissionEvaluator.IsAdministrator,
                new ResourceCapabilities(
                    alertCapabilities.CanRead || alertChannelCapabilities.CanRead,
                    alertCapabilities.CanWrite || alertChannelCapabilities.CanWrite,
                    alertCapabilities.CanExecute || alertChannelCapabilities.CanExecute),
                CapabilityMapper.ToResourceCapabilities(bindingPermissions),
                CapabilityMapper.ToResourceCapabilities(tagPermissions)),
            profile.CreatedAt,
            [.. profile.DirectRoles.Select(ProfileResourceInfoView.Map)],
            [.. profile.Teams.Select(ProfileResourceInfoView.Map)]);
    }
}

public sealed record CurrentProfileAuthorizationView(
    bool IsAdministrator,
    ResourceCapabilities AlertRules,
    ResourceCapabilities Bindings,
    ResourceCapabilities Tags);

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
