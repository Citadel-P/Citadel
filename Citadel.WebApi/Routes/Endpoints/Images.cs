using System.ComponentModel;
using System.Runtime.CompilerServices;
using Agent.Server.Images;
using Application.Features.Images.Queries;
using Hosting.Extensions;
using Infrastructure.GithubCr;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Registries;

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

    public static async Task<Results<Ok<IEnumerable<GhcrPackageVersion>>, ProblemHttpResult>> GetGhcrPackageVersions(IMediator mediator, [Description("The registry name")] string registryName, [Description("The package name")] string packageName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGithubPackageVersions(registryName, packageName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async IAsyncEnumerable<PullImageReply> PullImage(IMediator mediator, PullImageRequest pullImageRequest, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(pullImageRequest.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

}
