using Application.Features.Identity.Teams.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record DeleteTeamsInput(IEnumerable<Guid> Ids)
{
    internal DeleteTeams ToCommand() => new(Ids);
}
