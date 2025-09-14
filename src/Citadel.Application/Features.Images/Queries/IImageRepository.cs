using System.Text.Json.Serialization;
using Domain;

namespace Application.Features.Images.Queries;

[JsonPolymorphic]
[JsonDerivedType(typeof(GitHubPackageResponse), nameof(RegistryType.GitHub))]
[JsonDerivedType(typeof(DockerHubRepositoryResponse), nameof(RegistryType.DockerHub))]
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
    public string Name { get; init; } = null!;
    public string? Namespace { get; set; }
    public DateTime LastUpdated { get; set; }
    public bool IsPrivate { get; set; }
    public int PullCount { get; set; }
}