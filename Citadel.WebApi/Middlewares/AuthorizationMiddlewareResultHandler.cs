using Hosting.Common;
using Hosting.Middlewares;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Authorization.Policy;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Infrastructure;

namespace WebApi.Middlewares;

internal class AuthorizationResultHandler : IAuthorizationMiddlewareResultHandler
{
    public async Task HandleAsync(RequestDelegate next, HttpContext context, AuthorizationPolicy policy, PolicyAuthorizationResult authorizeResult)
    {
        if (authorizeResult.Succeeded)
        {
            await next(context);
        }
        else
        {
            context.Response.Clear();
            context.Response.StatusCode = authorizeResult.Forbidden
                ? StatusCodes.Status403Forbidden
                : StatusCodes.Status401Unauthorized;

            var problemFactory = context.RequestServices.GetRequiredService<ProblemDetailsFactory>();
            var problem = problemFactory.CreateProblemDetails(context, context.Response.StatusCode);

            await context.Response.WriteAsJsonAsync(
                            problem,
                            type: typeof(ProblemDetails),
                            context: ProblemDetailsSerializerContext.Default,
                            contentType: Constants.Api.ProblemContentType,
                            cancellationToken: context.RequestAborted);
        }
    }
}
