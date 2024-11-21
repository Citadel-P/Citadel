using Application.Features.Auth.Queries;

namespace WebApi.Controllers.V1.Resources.Auth;

public sealed record LoginRequest(string Email, string Password)
{
    internal LoginQuery ToQuery() => new(Email, Password);
}