using System.Text.Json.Serialization;
using Infrastructure;

namespace Application.Features.Images.Queries;

[JsonPolymorphic]
[JsonDerivedType(typeof(GitHubPackageResponse), nameof(RegistryDiscriminator.GitHub))]
[JsonDerivedType(typeof(DockerHubRepositoryResponse), nameof(RegistryDiscriminator.DockerHub))]
public interface IImageRepository
{
    string Name { get; }
}

public class GitHubPackageResponse : IImageRepository
{
    public string Id { get; init; } = null!;
    public string Name { get; init; } = null!;
    public string? CreatedAt { get; init; }
    public string? UpdatedAt { get; init; }
    public string? Url { get; init; }
    public string? HtmlUrl { get; init; }
}

public class DockerHubRepositoryResponse : IImageRepository
{
    public string? Name { get; init; }
    public string? Namespace { get; set; }
    public DateTime LastUpdated { get; set; }
    public bool IsPrivate { get; set; }
    public int PullCount { get; set; }
}