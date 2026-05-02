using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamView(Guid Id, string Name, Guid ActorId, bool IsEnabled, int TotalMembers, IEnumerable<string> Roles)
{
    internal static TeamView Map(TeamDetails team) => new(team.Id, team.Name, team.ActorId, team.IsEnabled, team.TotalMembers ?? 0, team.Roles ?? []);
}
