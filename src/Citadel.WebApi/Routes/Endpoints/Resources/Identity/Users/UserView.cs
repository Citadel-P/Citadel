using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UserView(
    Guid Id,
    string Name,
    string Email,
    Guid ActorId,
    bool IsEnabled,
    IEnumerable<ResourceInfo>? Teams = null,
    IEnumerable<ResourceInfo>? Roles = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null
    )
{
    internal static UserView Map(UserDetails user) => new(
        user.Id,
        user.Name,
        user.Email,
        user.ActorId,
        user.IsEnabled,
        user.Teams,
        user.Roles,
        user.ResourceAccesses);
}
