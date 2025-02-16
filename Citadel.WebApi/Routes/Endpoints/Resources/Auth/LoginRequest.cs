using Application.Features.Auth.Commands;

namespace WebApi.Routes.Endpoints.Resources.Auth;

public sealed record LoginRequest(string Email, string Password)
{
    internal LoginCommand ToQuery() => new(Email, Password);
}
