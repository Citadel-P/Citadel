using System.ComponentModel;
using Application.Features.Images.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Images;

namespace WebApi.Routes.Endpoints;

public static class Images
{
    public static async Task<Results<Ok<ImagesView>, ProblemHttpResult>> GetAllLocalImages(IMediator mediator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllLocalImages(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ImagesView.Map);
    }
    public static async Task<Results<Ok<IEnumerable<IImageResponse>>, ProblemHttpResult>> GetExternalImages(IMediator mediator, [Description("The registry name")] string registryName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetExternalImages(registryName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

}
