using Application.Features.Identity.Users.Commands;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record RemoveUserResourceAccessInput(ResourceType ResourceType, Guid ResourceId, ResourceAction Action)
{
    internal RemoveUserResourceAccess ToCommand(Guid userId) => new(userId, ResourceType, ResourceId, Action);
}
