using Application.Features.Identity.Users.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record DeleteUsersInput(IEnumerable<Guid> Ids)
{
    internal DeleteUsers ToCommand() => new(Ids);
}
