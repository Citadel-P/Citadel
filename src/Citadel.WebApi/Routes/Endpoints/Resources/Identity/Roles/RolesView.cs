using Domain.Contracts.Resources.Role;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record RolesView(IEnumerable<RoleView> Roles)
{
    internal static RolesView Map(IEnumerable<RoleDetails> roles) => new(roles.Select(RoleView.Map));
}
