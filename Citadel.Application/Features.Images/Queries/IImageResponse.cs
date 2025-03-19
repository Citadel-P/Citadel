using System.Text.Json.Serialization;
using Infrastructure;

namespace Application.Features.Images.Queries;

[JsonPolymorphic]
[JsonDerivedType(typeof(GitHubPackageResponse), nameof(RegistryDiscriminator.GitHub))]
[JsonDerivedType(typeof(DockerHubImageResponse), nameof(RegistryDiscriminator.DockerHub))]
public interface IImageResponse
{
    string Id { get; }
    string Name { get; }
}

public class GitHubPackageResponse : IImageResponse
{
    public string Id { get; init; }
    public string Name { get; init; }
    public string CreatedAt { get; init; }
    public string UpdatedAt { get; init; }
    public string Url { get; init; }
    public string HtmlUrl { get; init; }
}

public class DockerHubImageResponse : IImageResponse
{
    public string Id { get; init; }
    public string Name { get; init; }
}