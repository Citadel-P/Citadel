using Application.Features.Identity.Teams.Commands;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamResourceAccessInput(ResourceType ResourceType, Guid ResourceId, ResourceAction Action)
{
    internal TeamResourceAccessModel ToModel() => new(ResourceType, ResourceId, Action);
}

public sealed record CreateTeamInput(
    string Name,
    IEnumerable<Guid>? UserIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<TeamResourceAccessInput>? ResourceAccesses = null)
{
    internal CreateTeam ToCommand() => new(
        Name,
        UserIds,
        RoleIds,
        ResourceAccesses?.Select(x => x.ToModel()));
}
