using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using Agent.Server.Images;
using Google.Protobuf.Collections;
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

public record DockerHubPublicImage (string Name, string Icon, string Url, string Description);

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(List<DockerHubPublicImage>))]
internal partial class DockerHubPublicImageContext : JsonSerializerContext
{
}