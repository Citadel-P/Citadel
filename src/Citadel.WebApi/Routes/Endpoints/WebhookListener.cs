using Application.Features.Webhooks.Commands;
using Hosting.Common.ErrorTypes;
using Mediator;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints;

public static class WebhookListener
{
    public static async Task<IResult> Receive(
        HttpContext httpContext,
        IMediator mediator,
        [FromRoute] string authType,
        [FromRoute] string resourceType,
        [FromRoute] Guid id,
        [FromRoute] string execution,
        CancellationToken cancellationToken)
    {
        var command = new ReceiveWebhook(
            authType,
            resourceType,
            id,
            execution,
            ToHeaders(httpContext.Request.Headers),
            await ReadRawBodyAsync(httpContext.Request, cancellationToken));

        var result = await mediator.Send(command, cancellationToken);
        return ToHttpResult(result);
    }

    private static IResult ToHttpResult(LightResults.Result<WebhookReceiveResult> result)
    {
        if (result.IsSuccess(out var response))
            return Results.Accepted(value: response);

        _ = result.IsFailure(out var error);
        return error switch
        {
            NotFoundError => Results.NotFound(),
            UnauthorizedError => Results.Unauthorized(),
            BadRequestError => Results.BadRequest(new WebhookReceiveResult(false, "rejected", Guid.CreateVersion7(), error.Message)),
            _ => Results.Problem(error?.Message ?? "Webhook request failed.")
        };
    }

    private static Dictionary<string, string[]> ToHeaders(IHeaderDictionary headers)
        => headers.ToDictionary(
            pair => pair.Key,
            pair => pair.Value.Where(value => value is not null).Select(value => value!).ToArray(),
            StringComparer.OrdinalIgnoreCase);

    private static async Task<byte[]> ReadRawBodyAsync(HttpRequest request, CancellationToken cancellationToken)
    {
        using var stream = new MemoryStream();
        await request.Body.CopyToAsync(stream, cancellationToken);
        return stream.ToArray();
    }
}
