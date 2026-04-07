using Application.Features.Identity.Teams.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record CreateTeamInput(string Name)
{
    internal CreateTeam ToCommand() => new(Name);
}
