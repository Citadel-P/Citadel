using Application.Models;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.DependencyInjection;

namespace WebApi.Middlewares;

internal sealed class SetupRequiredMiddleware(
    RequestDelegate next,
    ISetupStateCache cache)
{
    private static readonly PathString SetupPath = new("/api/v1/setup");

    public async Task InvokeAsync(HttpContext context)
    {
        if (!IsGatedPath(context.Request.Path)
            || context.Request.Path.StartsWithSegments(
                SetupPath,
                StringComparison.OrdinalIgnoreCase))
        {
            await next(context);
            return;
        }

        bool requiresSetup;
        if (!cache.TryGetRequiresSetup(out requiresSetup))
        {
            var unitOfWork =
                context.RequestServices.GetRequiredService<IUnitOfWork>();
            var state = await unitOfWork.InstanceSetupState.GetAsync(
                context.RequestAborted);
            if (state is null)
            {
                await WriteProblemAsync(
                    context,
                    StatusCodes.Status503ServiceUnavailable,
                    "setup_unavailable",
                    "Citadel setup state is unavailable.");
                return;
            }

            requiresSetup = state.RequiresSetup;
            cache.SetRequiresSetup(requiresSetup);
        }

        if (!requiresSetup)
        {
            await next(context);
            return;
        }

        await WriteProblemAsync(
            context,
            StatusCodes.Status409Conflict,
            "setup_required",
            "Citadel must be initialized before this endpoint can be used.");
    }

    private static bool IsGatedPath(PathString path)
        => path.StartsWithSegments("/api/v1", StringComparison.OrdinalIgnoreCase)
            || path.StartsWithSegments("/hubs", StringComparison.OrdinalIgnoreCase)
            || path.StartsWithSegments("/listener", StringComparison.OrdinalIgnoreCase);

    private static Task WriteProblemAsync(
        HttpContext context,
        int status,
        string type,
        string detail)
    {
        context.Response.StatusCode = status;
        context.Response.Headers.CacheControl = "no-store";
        return context.Response.WriteAsJsonAsync(
            new ProblemDetails
            {
                Type = type,
                Title = status == StatusCodes.Status409Conflict
                    ? "Setup required"
                    : "Setup unavailable",
                Status = status,
                Detail = detail
            },
            ProblemJsonContext.Default.ProblemDetails);
    }
}
