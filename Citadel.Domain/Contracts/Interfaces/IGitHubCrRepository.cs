using Domain.Contracts.Resources.Registries;
using Domain.Entities.Registries;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Provides methods for interacting with GitHub Container Registry (ghcr.io).
/// </summary>
public interface IGitHubCrRepository
{
    Task<(bool success, string? errorMessage)> CanConnectAsync(GitHubRegistry gitHubRegistry, CancellationToken cancellationToken);
    Task<(IEnumerable<GitHubCrPackage>? packages, string? errorMessage)> GetPackagesAsync(GitHubRegistry gitHubRegistry, CancellationToken cancellationToken);
    Task<(IEnumerable<GitHubCrPackageVersion>? versions, string? errorMessage)> GetPackageVersionsAsync(GitHubRegistry gitHubRegistry, string packageName, CancellationToken cancellationToken);
}

