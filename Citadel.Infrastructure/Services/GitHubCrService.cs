using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Registries;
using Infrastructure.GithubCr;
using Refit;

namespace Infrastructure.Services;

internal class GitHubCrService(IGithubCrApi gitHubCrApi) : IGitHubCrService
{
    public async Task<(bool success, string? errorMessage)> CanConnectAsync(GitHubRegistry gitHubRegistry, CancellationToken cancellationToken)
    {
        try
        {
            var packages = gitHubRegistry.Type == GhcrAccountType.Organization
                    ? await gitHubCrApi.ListOrgPackages(gitHubRegistry.Name, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken)
                    : await gitHubCrApi.ListUserPackages(gitHubRegistry.Name, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken);
            return (true, null);
        }
        catch (ApiException ex)
        {
            return (false,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid GitHub credentials, please verify your input."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<GitHubCrPackage>? packages, string? errorMessage)> GetPackagesAsync(GitHubRegistry gitHubRegistry, CancellationToken cancellationToken)
    {
        try
        {
            var packages = gitHubRegistry.Type == GhcrAccountType.User
                ? await gitHubCrApi.ListUserPackages(gitHubRegistry.Name, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken)
                : await gitHubCrApi.ListOrgPackages(gitHubRegistry.Name, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken);
            return (packages.Map(), null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<GitHubCrPackageVersion>? versions, string? errorMessage)> GetPackageVersionsAsync(GitHubRegistry gitHubRegistry, string packageName, CancellationToken cancellationToken)
    {
        try
        {
            var versions = gitHubRegistry.Type == GhcrAccountType.User
                        ? await gitHubCrApi.ListPackageVersionsForUser(packageName, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken: cancellationToken)
                        : await gitHubCrApi.ListPackageVersionsForOrg(gitHubRegistry.Name, packageName, gitHubRegistry.PAT, gitHubRegistry.Name, cancellationToken: cancellationToken);
            return (versions.Map(), null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }
}

internal static class GitHubServiceMapper
{
    public static IEnumerable<GitHubCrPackage> Map(this IEnumerable<GhcrPackage> packages) 
        => packages.Select(Map);

    public static GitHubCrPackage Map(this GhcrPackage package)
        => new (
            Id: package.Id,
            Name: package.Name,
            CreatedAt: package.CreatedAt,
            UpdatedAt: package.UpdatedAt,
            PackageType: package.PackageType,
            Url: package.Url,
            VersionCount: package.VersionCount,
            HtmlUrl: package.HtmlUrl
        );

    public static IEnumerable<GitHubCrPackageVersion> Map(this IEnumerable<GhcrPackageVersion> versions)
        => versions.Select(Map);

    public static GitHubCrPackageVersion Map(this GhcrPackageVersion version)
        => new (
            Id: version.Id,
            Name: version.Name,
            Url: version.Url,
            PackageHtmlUrl: version.PackageHtmlUrl,
            CreatedAt: version.CreatedAt,
            UpdatedAt: version.UpdatedAt,
            HtmlUrl: version.HtmlUrl,
            Metadata: version.Metadata is not null
                ? new GitHubCrPackageVersionMetadata(
                    Container: version.Metadata.Container is not null
                        ? new GitHubCrPackageVersionContainerMetadata(Tags: version.Metadata.Container.Tags)
                        : null)
                : null
        );
}
