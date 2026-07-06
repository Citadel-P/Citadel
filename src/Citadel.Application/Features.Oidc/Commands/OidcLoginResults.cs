using Application.Features.Identity.Auth.Models;

namespace Application.Features.Oidc.Commands;

public sealed record OidcLoginStartResult(string AuthorizationUrl);

public sealed record OidcLoginCompleteResult(LoginResponse Login, string ReturnUrl);
