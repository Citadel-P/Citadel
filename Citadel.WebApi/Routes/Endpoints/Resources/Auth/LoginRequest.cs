using Application.Features.Auth.Queries;

namespace WebApi.Routes.Endpoints.Resources.Auth;

public sealed record LoginRequest(string Email, string Password)
{
    internal LoginQuery ToQuery() => new(Email, Password);
}