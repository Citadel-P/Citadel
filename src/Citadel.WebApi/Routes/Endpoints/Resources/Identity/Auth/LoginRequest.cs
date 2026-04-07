using Application.Features.Identity.Auth.Commands;

namespace WebApi.Routes.Endpoints.Resources.Identity.Auth;

public sealed record LoginRequest(string EmailOrName, string Password)
{
    internal LoginCommand ToQuery() => new(EmailOrName, Password);
}
