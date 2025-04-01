using System.Text.Json;
using System.Text.Json.Serialization;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetDockerHubPublicImages(string ImageName): IQuery<Result<IEnumerable<DockerHubPublicImage>>>;

internal sealed class GetDockerHubPublicImagesHandler : IQueryHandler<GetDockerHubPublicImages, Result<IEnumerable<DockerHubPublicImage>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubPublicImage>>> Handle(GetDockerHubPublicImages query, CancellationToken cancellationToken)
    {
        if (string.IsNullOrEmpty(query.ImageName))
        {
            using FileStream openStream = File.OpenRead(Constants.DefaultImagesDefinitionsPath);
            return await JsonSerializer.DeserializeAsync(openStream, DockerHubPublicImageContext.Default.ListDockerHubPublicImage,  cancellationToken);
        }
        else
        {
            return new List<DockerHubPublicImage>();
        }
    }
}

public record DockerHubPublicImage
{
    [JsonPropertyName("name")] public string Name { get; init; }
    [JsonPropertyName("icon")] public string Icon { get; init; }
    [JsonPropertyName("url")] public string Url { get; init; }
    [JsonPropertyName("description")] public string Description { get; init; }

}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(List<DockerHubPublicImage>))]
internal partial class DockerHubPublicImageContext : JsonSerializerContext
{
}