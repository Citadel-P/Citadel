using System.Text.Json.Serialization;
using Infrastructure.DockerHub;
using Refit;

namespace Infrastructure.Entities.Registries;

[method: JsonConstructor]
public record DockerHubRegistry(string? UserName = null, string? PAT = null) : RegistryConfigurationBase
{
    public override string RegistryUrl => "https://docker.io";
    public static DockerHubRegistry Create(string userName, string PAT) => new (userName, PAT);

    public async Task<(bool success, string? errorMessage)> CanConnect(IDockerHubApi dockerHub, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            return (true, null);
        }
        catch (ApiException ex)
        {
            return (false, 
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<DockerHubRepository>? repositories, string? errorMessage)> GetRepositories(IDockerHubApi dockerHub, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            var repositories = await dockerHub.GetRepositories(UserName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories.Results, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(ICollection<Tag>? tags, string? errorMessage)> GetRepositoryTags(IDockerHubApi dockerHub, string repositoryName, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = UserName, Secret = PAT }, cancellationToken);
            var repositories = await dockerHub.TagsGET(UserName, repositoryName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories.Results, null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public override string GetRegistryAuth() => new RegistryAuth(UserName, PAT, RegistryUrl).GetAuth();
}

