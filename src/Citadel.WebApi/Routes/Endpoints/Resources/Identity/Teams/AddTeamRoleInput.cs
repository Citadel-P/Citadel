using Application.Features.Identity.Teams.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record AddTeamRoleInput(Guid RoleId)
{
    internal AddTeamRole ToCommand(Guid teamId) => new(teamId, RoleId);
}
