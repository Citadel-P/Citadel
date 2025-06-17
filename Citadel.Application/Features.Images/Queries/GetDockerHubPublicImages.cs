using System.Text.Json;
using System.Text.Json.Serialization;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetDockerHubPublicImages(string? ImageName): IQuery<Result<IEnumerable<DockerHubImageResult>>>;

internal sealed class GetDockerHubPublicImagesHandler(IDockerHubRegistryRepository dockerHubRegistryRepository) : IQueryHandler<GetDockerHubPublicImages, Result<IEnumerable<DockerHubImageResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubImageResult>>> Handle(GetDockerHubPublicImages query, CancellationToken cancellationToken)
    {
        
        if (string.IsNullOrEmpty(query.ImageName))
        {
            try
            {
                using FileStream openStream = File.OpenRead(Constants.DefaultImagesDefinitionsPath);
                return (await JsonSerializer.DeserializeAsync(openStream, DockerHubPublicImageContext.Default.ListDockerHubImageResult, cancellationToken) ?? []);
            }
            catch (JsonException ex)
            {
                return Result.Failure<IEnumerable<DockerHubImageResult>>(new InternalServerError(ex.Message));
            }
        }
        else
        {
            var result = await dockerHubRegistryRepository.SearchImage(query.ImageName, cancellationToken);
            if (result.errorMessage is not null)
            {
                return Result.Failure<IEnumerable<DockerHubImageResult>>(new InternalServerError(result.errorMessage));
            }
            else return result.images?.ToList() ?? [];
        }
    }
}


[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(List<DockerHubImageResult>))]
internal partial class DockerHubPublicImageContext : JsonSerializerContext
{
}