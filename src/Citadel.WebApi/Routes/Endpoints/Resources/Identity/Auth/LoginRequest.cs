using Application.Features.Auth.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Auth;

public sealed record LoginRequest(string Email, string Password)
{
    internal LoginCommand ToQuery() => new(Email, Password);
}
