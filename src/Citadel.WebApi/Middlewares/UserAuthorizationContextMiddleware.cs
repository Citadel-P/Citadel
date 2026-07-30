using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;

namespace WebApi.Middlewares;

internal sealed class UserAuthorizationContextMiddleware(RequestDelegate next)
{
    public async Task InvokeAsync(
        HttpContext context,
        IUnitOfWork unitOfWork,
        IRoleCache roleCache)
    {
        if (context.User.Identity?.IsAuthenticated == true
            && TryGetUserId(context.User, out var userId)
            && roleCache.GetRoles(userId) is null)
        {
            var user = await unitOfWork.Users.GetUserAuthInfoByIdAsync(userId, context.RequestAborted);
            if (user is null)
            {
                context.User = new ClaimsPrincipal(new ClaimsIdentity());
            }
            else
            {
                roleCache.SetRoles(userId, user.Roles);
            }
        }

        await next(context);
    }

    private static bool TryGetUserId(ClaimsPrincipal principal, out Guid userId)
    {
        var value = principal.FindFirst(JwtRegisteredClaimNames.Sub)?.Value
            ?? principal.FindFirst(ClaimTypes.NameIdentifier)?.Value;
        return Guid.TryParse(value, out userId);
    }
}
