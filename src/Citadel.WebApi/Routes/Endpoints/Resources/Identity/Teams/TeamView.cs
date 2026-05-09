using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamView(
    Guid Id, 
    string Name, 
    Guid ActorId, 
    bool IsEnabled, 
    int TotalMembers, 
    IEnumerable<ResourceInfo>? Users = null,
    IEnumerable<ResourceInfo>? Roles = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null)
{
    internal static TeamView Map(TeamDetails team) => new(team.Id, team.Name, team.ActorId, team.IsEnabled, team.TotalMembers ?? 0, team.Users, team.Roles, team.ResourceAccesses);
}
