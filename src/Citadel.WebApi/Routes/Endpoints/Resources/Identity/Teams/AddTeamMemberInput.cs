using Application.Features.Identity.Teams.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record AddTeamMemberInput(Guid MemberActorId)
{
    internal AddTeamMember ToCommand(Guid teamId) => new(teamId, MemberActorId);
}
