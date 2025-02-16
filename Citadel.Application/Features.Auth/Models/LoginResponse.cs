using Infrastructure;

namespace Application.Features.Auth.Models;

public sealed record LoginResponse(string AccessToken, IEnumerable<AppPermission> Permissions);