using Infrastructure;

namespace Application.Features.Auth.Models;
public sealed record LoginResponse(string Jwt, AppPermission[] Permissions);