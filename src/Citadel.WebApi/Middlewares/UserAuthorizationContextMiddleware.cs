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
            && TryGetActorId(context.User, out var actorId)
            && roleCache.GetRoles(actorId) is null)
        {
            var roles = await unitOfWork.Actors.GetRoleNamesAsync(actorId, context.RequestAborted);
            if (roles is null)
            {
                context.User = new ClaimsPrincipal(new ClaimsIdentity());
            }
            else
            {
                roleCache.SetRoles(actorId, roles);
            }
        }

        await next(context);
    }

    private static bool TryGetActorId(ClaimsPrincipal principal, out Guid actorId)
    {
        return Guid.TryParse(principal.FindFirst("actorId")?.Value, out actorId);
    }
}
