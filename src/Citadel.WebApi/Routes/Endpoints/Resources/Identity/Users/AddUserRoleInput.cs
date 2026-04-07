using Application.Features.Identity.Users.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record AddUserRoleInput(Guid RoleId)
{
    internal AddUserRole ToCommand(Guid userId) => new(userId, RoleId);
}
