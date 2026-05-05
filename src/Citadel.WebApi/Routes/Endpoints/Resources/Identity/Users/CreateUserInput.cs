using Application.Features.Identity.Users.Commands;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UserResourceAccessInput(ResourceType ResourceType, Guid ResourceId, ResourceAction Action)
{
    internal UserResourceAccessModel ToModel() => new(ResourceType, ResourceId, Action);
}

public sealed record CreateUserInput(
    string Name,
    string Email,
    string Password,
    bool IsEnabled = true,
    IEnumerable<Guid>? TeamIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<UserResourceAccessInput>? ResourceAccesses = null)
{
    internal CreateUser ToCommand() => new(
        Name,
        Email,
        Password,
        IsEnabled,
        TeamIds,
        RoleIds,
        ResourceAccesses?.Select(x => x.ToModel()));
}
