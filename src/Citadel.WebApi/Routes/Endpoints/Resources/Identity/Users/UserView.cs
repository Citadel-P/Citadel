using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UserView(
    Guid Id,
    string Name,
    string Email,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    IEnumerable<string>? Teams = null,
    IEnumerable<string>? Roles = null
    )
{
    internal static UserView Map(UserDetails user) => new(
        user.Id,
        user.Name,
        user.Email,
        user.ActorId,
        user.IsEnabled,
        user.CreatedAt,
        user.CreatedByActorId,
        user.Teams,
        user.Roles);
}
