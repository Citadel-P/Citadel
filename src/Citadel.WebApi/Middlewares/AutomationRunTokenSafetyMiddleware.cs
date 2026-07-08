using Application.Models;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Middlewares;

internal sealed class AutomationRunTokenSafetyMiddleware(RequestDelegate next)
{
    private static readonly PathString[] BlockedPrefixes =
    [
        new("/api/v1/authentication"),
        new("/api/v1/automation"),
        new("/api/v1/resourceBindings/secrets"),
        new("/api/v1/resourceBindings/secret-providers")
    ];

    public async Task InvokeAsync(HttpContext context)
    {
        if (context.User.HasClaim(static claim => claim.Type == "automationRunId") && IsBlocked(context.Request.Path))
        {
            context.Response.StatusCode = StatusCodes.Status403Forbidden;
            await context.Response.WriteAsJsonAsync(
                new ProblemDetails
                {
                    Title = "Forbidden",
                    Status = StatusCodes.Status403Forbidden,
                    Detail = "Automation run tokens cannot call this endpoint."
                },
                ProblemJsonContext.Default.ProblemDetails);
            return;
        }

        await next(context);
    }

    private static bool IsBlocked(PathString path)
    {
        foreach (var prefix in BlockedPrefixes)
        {
            if (path.StartsWithSegments(prefix, StringComparison.OrdinalIgnoreCase))
                return true;
        }

        var value = path.Value ?? string.Empty;
        return value.Contains("/terminal", StringComparison.OrdinalIgnoreCase)
            || value.Contains("/exec", StringComparison.OrdinalIgnoreCase);
    }
}
