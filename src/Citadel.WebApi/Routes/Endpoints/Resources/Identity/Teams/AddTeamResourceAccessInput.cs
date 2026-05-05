using Application.Features.Identity.Teams.Commands;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record AddTeamResourceAccessInput(ResourceType ResourceType, Guid ResourceId, ResourceAction Action)
{
    internal AddTeamResourceAccess ToCommand(Guid teamId) => new(teamId, ResourceType, ResourceId, Action);
}
