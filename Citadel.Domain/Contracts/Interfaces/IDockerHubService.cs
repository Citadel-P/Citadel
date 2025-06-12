using Domain.Contracts.Resources.Registries;
using Domain.Entities.Registries;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Provides methods for interacting with Docker Hub registries.
/// </summary>
public interface IDockerHubService
{
    Task<(bool success, string? errorMessage)> CanConnectAsync(DockerHubRegistry dockerHubRegistry, CancellationToken cancellationToken);
    Task<(IEnumerable<DockerHubRepositoryInfo>? repositories, string? errorMessage)> GetRepositoriesAsync(DockerHubRegistry dockerHubRegistry, CancellationToken cancellationToken);
    Task<(IEnumerable<DockerHubTag>? tags, string? errorMessage)> GetRepositoryTagsAsync(DockerHubRegistry dockerHubRegistry, string repositoryName, CancellationToken cancellationToken);
}

