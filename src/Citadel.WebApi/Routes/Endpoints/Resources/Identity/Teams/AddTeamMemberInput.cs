using Application.Features.Identity.Teams.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record AddTeamMemberInput(Guid UserId)
{
    internal AddTeamMember ToCommand(Guid teamId) => new(teamId, UserId);
}
