using System.Text.Json.Serialization;
using Infrastructure.GithubCr;
using Refit;

namespace Infrastructure.Entities.Registries;

[method: JsonConstructor]
public record GitHubRegistry(string Name, string PAT, GhcrAccountType? Type) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://ghcr.io";

    public static GitHubRegistry Create(string name, string PAT, GhcrAccountType type) =>
        new (name, PAT, type);

    public async Task<(bool success, string? errorMessage)> CanConnect(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
    {
        try
        {
            var packages = Type == GhcrAccountType.Organization
                    ? await githubCrApi.ListOrgPackages(Name, PAT, Name, cancellationToken)
                    : await githubCrApi.ListUserPackages(Name, PAT, Name, cancellationToken);
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

    public async Task<(IEnumerable<GhcrPackage>? packages, string? errorMessage)> GetPackages(IGithubCrApi githubCrApi, CancellationToken cancellationToken)
    {
        try
        {
            var packages = Type == GhcrAccountType.User
                ? await githubCrApi.ListUserPackages(Name, PAT, Name, cancellationToken)
                : await githubCrApi.ListOrgPackages(Name, PAT, Name, cancellationToken);
            return (packages, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<GhcrPackageVersion>? versions, string? errorMessage)> GetPackageVersions(IGithubCrApi githubCrApi, string packageName, CancellationToken cancellationToken)
    {
        try
        {
            var versions = Type == GhcrAccountType.User
                        ? await githubCrApi.ListPackageVersionsForUser(packageName, PAT, Name, cancellationToken: cancellationToken)
                        : await githubCrApi.ListPackageVersionsForOrg(Name, packageName, PAT, Name, cancellationToken: cancellationToken);
            return (versions, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public override string GetRegistryAuth() => new RegistryAuth(Name, PAT, RegistryUrl).GetAuth();
}

