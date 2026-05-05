using Application.Features.Identity.Teams.Commands;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record RemoveTeamResourceAccessInput(ResourceType ResourceType, Guid ResourceId, ResourceAction Action)
{
    internal RemoveTeamResourceAccess ToCommand(Guid teamId) => new(teamId, ResourceType, ResourceId, Action);
}
