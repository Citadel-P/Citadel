using Application.Models;
using Hosting.Common;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Middlewares;

internal sealed class ServiceAccountTokenSafetyMiddleware(RequestDelegate next)
{
    private static readonly PathString[] BlockedPrefixes =
    [
        new("/api/v1/authentication"),
        new("/api/v1/profile"),
        new("/hubs"),
    ];

    public async Task InvokeAsync(HttpContext context)
    {
        if (IsServiceAccount(context.User) && IsBlocked(context.Request.Path))
        {
            context.Response.StatusCode = StatusCodes.Status403Forbidden;
            await context.Response.WriteAsJsonAsync(
                new ProblemDetails
                {
                    Title = "Forbidden",
                    Status = StatusCodes.Status403Forbidden,
                    Detail = "Service Account credentials cannot call this endpoint.",
                },
                ProblemJsonContext.Default.ProblemDetails);
            return;
        }

        await next(context);
    }

    private static bool IsServiceAccount(System.Security.Claims.ClaimsPrincipal principal)
        => string.Equals(
            principal.FindFirst("principalType")?.Value,
            AuthenticatedPrincipalType.ServiceAccount.ToString(),
            StringComparison.OrdinalIgnoreCase);

    private static bool IsBlocked(PathString path)
        => BlockedPrefixes.Any(prefix => path.StartsWithSegments(prefix, StringComparison.OrdinalIgnoreCase));
}
