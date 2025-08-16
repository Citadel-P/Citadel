using System.ComponentModel;
using System.Runtime.CompilerServices;
using Application.Features.Images.Queries;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Registries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Images;
using PullImageRequest = WebApi.Routes.Endpoints.Resources.Images.PullImageRequest;

namespace WebApi.Routes.Endpoints;

public static class Images
{
    public static async Task<Results<Ok<ImagesView>, ProblemHttpResult>> GetAllLocalImages(IMediator mediator, [Description("The platform id")] Guid platformId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllLocalImages(platformId), cancellationToken);
        return EndpointHandlers.HandleResult(result, ImagesView.Map);
    }

    public static async Task<Results<Ok<IEnumerable<IImageRepository>>, ProblemHttpResult>> GetExternalRepositories(IMediator mediator, [Description("The registry name")] string registryName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetExternalRepositories(registryName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<IEnumerable<GitHubCrPackageVersion>>, ProblemHttpResult>> GetGhcrPackageVersions(IMediator mediator, [Description("The registry name")] string registryName, [Description("The package name")] string packageName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGithubPackageVersions(registryName, packageName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<IEnumerable<DockerHubTagView>>, ProblemHttpResult>> GetDockerHubRepositoryTags(IMediator mediator, [Description("The registry name")] string registryName, [Description("The repository name")] string repositoryName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDockerHubRepositoryTags(registryName, repositoryName), cancellationToken);
        return EndpointHandlers.HandleResult(result, Mapper.Map);
    }

    public static async Task<Results<Ok<IEnumerable<DockerHubRepositoryInfo>>, ProblemHttpResult>> GetDockerHubRepositories(IMediator mediator, [Description("The registry name")] string registryName, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDockerHubRepositories(registryName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<IEnumerable<DockerHubImageResult>>, ProblemHttpResult>> GetDockerHubPublicImages(IMediator mediator, [FromQuery] string? imageName = null, CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetDockerHubPublicImages(imageName), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async IAsyncEnumerable<PullImageResult> PullImage(IMediator mediator, PullImageRequest pullImageRequest, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(pullImageRequest.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async Task<Results<Ok<DeleteImageResult>, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteImagesRequest deleteImagesRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(deleteImagesRequest.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<InspectImageResult>, ProblemHttpResult>> Inspect(IMediator mediator, Guid platformId, string imageId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectImage(platformId, imageId), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<ImageInfoResult>, ProblemHttpResult>> GetImageInfo(IMediator mediator, Guid platformId, string imageId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetImageInfo(platformId, imageId), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

}
