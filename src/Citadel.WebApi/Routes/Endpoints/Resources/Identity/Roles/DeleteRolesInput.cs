using Application.Features.Identity.Roles.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Roles;

public sealed record DeleteRolesInput(IEnumerable<Guid> Ids)
{
    internal DeleteRoles ToCommand() => new(Ids);
}
