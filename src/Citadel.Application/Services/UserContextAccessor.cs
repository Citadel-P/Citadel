using Hosting.Common.Abstraction;
using Application.Services.Identity;
using Hosting.Common.Extensions;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
using System.IdentityModel.Tokens.Jwt;
using Hosting.Common;

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

        var actorId = user.GetActorId();
        var principalType = ParsePrincipalType(user.FindFirst("principalType")?.Value);
        var principalResourceId = TryGetSubject(user, out var subjectId) ? subjectId : actorId;
        var userId = principalType == AuthenticatedPrincipalType.User ? principalResourceId : Guid.Empty;
        var credentialId = Guid.TryParse(user.FindFirst("credentialId")?.Value, out var parsedCredentialId)
            ? parsedCredentialId
            : (Guid?)null;
        var roles = roleCache.GetRoles(actorId) ?? [];
        var isAdmin = IsAdmin(roles);
        var isAuthenticated = user.Identity?.IsAuthenticated == true;
        return new UserContext
        {
            UserId = userId,
            ActorId = actorId,
            PrincipalType = principalType,
            PrincipalResourceId = principalResourceId,
            CredentialId = credentialId,
            IsAdmin = isAdmin,
            Roles = roles,
            IsAuthenticated = isAuthenticated
        };
    }

    private static AuthenticatedPrincipalType ParsePrincipalType(string? value)
        => Enum.TryParse<AuthenticatedPrincipalType>(value, true, out var result)
            ? result
            : AuthenticatedPrincipalType.User;

    private static bool TryGetSubject(ClaimsPrincipal principal, out Guid id)
    {
        var value = principal.FindFirst(JwtRegisteredClaimNames.Sub)?.Value
            ?? principal.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return Guid.TryParse(value, out id);
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
        public Guid ActorId { get; init; }
        public AuthenticatedPrincipalType PrincipalType { get; init; }
        public Guid PrincipalResourceId { get; init; }
        public Guid? CredentialId { get; init; }
        public bool IsAdmin { get; init; }
        public bool IsAuthenticated { get; init; }
        public string[] Roles { get; init; } = [];

    }
}
