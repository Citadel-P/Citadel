using System.Text.Json;
using System.Text.Json.Serialization;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.DockerHub;
using LightResults;
using Mediator;
using Refit;

namespace Application.Features.Images.Queries;

public sealed record GetDockerHubPublicImages(string? ImageName): IQuery<Result<IEnumerable<DockerHubImageModel>>>;

internal sealed class GetDockerHubPublicImagesHandler(IDockerHubApi dockerHub) : IQueryHandler<GetDockerHubPublicImages, Result<IEnumerable<DockerHubImageModel>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubImageModel>>> Handle(GetDockerHubPublicImages query, CancellationToken cancellationToken)
    {
        
        if (string.IsNullOrEmpty(query.ImageName))
        {
            try
            {
                using FileStream openStream = File.OpenRead(Constants.DefaultImagesDefinitionsPath);
                return (await JsonSerializer.DeserializeAsync(openStream, DockerHubPublicImageContext.Default.ListDockerHubImageModel, cancellationToken) ?? []);
            }
            catch (JsonException ex)
            {
                return Result.Failure<IEnumerable<DockerHubImageModel>>(new InternalServerError(ex.Message));
            }
        }
        else
        {
            try
            {
                var pagedResult = await dockerHub.SearchImage(query.ImageName, cancellationToken);
                return pagedResult.Results?.OrderByDescending(s => s.StarCount).ThenByDescending(s => s.PullCount).Select(Mapper.Map).ToList() ?? [];
            }
            catch (ApiException ex)
            {
                return Result.Failure<IEnumerable<DockerHubImageModel>>(new InternalServerError(ex.Message));
            }
        }
    }
}

internal partial class Mapper
{
    public static DockerHubImageModel Map(DockerHubImageModel image) => new ()
    {
        IsOfficial = image.IsOfficial,
        Name = image.Name,
        PullCount = image.PullCount,
        StarCount = image.StarCount,
        Url = image.IsOfficial ? $"https://hub.docker.com/_/{image.Name}" : $"https://hub.docker.com/r/{image.Name}",
        Description = image.Description,
    };
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(List<DockerHubImageModel>))]
internal partial class DockerHubPublicImageContext : JsonSerializerContext
{
}