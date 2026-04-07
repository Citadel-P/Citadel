using Application.Features.Identity.Users.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record CreateUserInput(string Name, string Email, string Password)
{
    internal CreateUser ToCommand() => new(Name, Email, Password);
}
