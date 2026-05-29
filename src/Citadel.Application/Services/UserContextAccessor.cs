using Hosting.Common.Abstraction;
using Application.Services.Identity;
using Hosting.Common.Extensions;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Services;

internal sealed class UserContextAccessor : IUserContextAccessor
{
    public IUserContext Current { get; }

    public UserContextAccessor(IHttpContextAccessor accessor, IRoleCache roleCache)
    {
        var user = accessor.HttpContext?.User;

        Current = Build(user, roleCache);
    }

    private static UserContext Build(ClaimsPrincipal? user, IRoleCache roleCache)
    {
        if (user is null || !user.Identity?.IsAuthenticated == true)
        {
            return new UserContext();
        }

        var userId = user.GetUserId();
        var roles = roleCache.GetRoles(userId) ?? [];
        var isAdmin = IsAdmin(roles);
        var isAuthenticated = user.Identity?.IsAuthenticated == true;
        return new UserContext
        {
            UserId = userId,
            IsAdmin = isAdmin,
            Roles = roles,
            IsAuthenticated = isAuthenticated
        };
    }

    public static bool IsAdmin(string[] roles)
    {
        foreach (var role in roles)
        {
            if (string.Equals(role, "admin", StringComparison.OrdinalIgnoreCase))
                return true;
        }

        return false;
    }

    internal class UserContext : IUserContext
    {
        public Guid UserId { get; init; }
        public bool IsAdmin { get; init; }
        public bool IsAuthenticated { get; init; }
        public string[] Roles { get; init; } = [];
    }
}