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

namespace WebApi.Routes.Endpoints;

public static class Images
{
    public static async Task<Results<Ok<ImagesView>, ProblemHttpResult>> ListImages(IMediator mediator, [Description("The platform id")] Guid platformId, CancellationToken cancellationToken)
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

    public static async IAsyncEnumerable<PullImageStreamItem> PullImage(IMediator mediator, PullImageInput pullImageInput, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(pullImageInput.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async Task<Results<Ok<DeleteImageResult>, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteImagesRequest deleteImagesRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(deleteImagesRequest.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

    public static async Task<Results<Ok<InspectImageView>, ProblemHttpResult>> Inspect(IMediator mediator, Guid platformId, string imageId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectImage(platformId, imageId), cancellationToken);
        return EndpointHandlers.HandleResult(result, InspectImageView.Map);
    }

    public static async Task<Results<Ok<ExposedPortsResult>, ProblemHttpResult>> GetExposedPorts(IMediator mediator, Guid platformId, string imageId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetExposedPorts(platformId, imageId), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => v);
    }

}
