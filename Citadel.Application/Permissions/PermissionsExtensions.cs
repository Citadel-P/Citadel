using System.Security.Claims;
using Infrastructure;

namespace Application.Permissions;

internal static class PermissionsExtensions
{
    internal static bool IsAdmin(this ClaimsPrincipal user)
        => user.IsInRole("Admin");

    internal static bool HasPermission(this ClaimsPrincipal user, AppPermission permission) 
        => user.Claims.Any(c => c.Type == "permissions" && c.Value.Contains(permission.ToString()));
}
