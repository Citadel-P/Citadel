using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Registries;
using Infrastructure.DockerHub;
using Infrastructure.Repositories.Mappers;
using Refit;

namespace Infrastructure.Repositories;

internal class DockerHubRegistryRepository(IDockerHubApi dockerHub) : IDockerHubRegistryRepository
{
    public async Task<(bool success, string? errorMessage)> CanConnectAsync(DockerHubRegistry dockerHubRegistry, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = dockerHubRegistry?.UserName, Secret = dockerHubRegistry?.PAT }, cancellationToken);
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

    public async Task<(IEnumerable<DockerHubRepositoryInfo>? repositories, string? errorMessage)> GetRepositoriesAsync(DockerHubRegistry dockerHubRegistry, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = dockerHubRegistry?.UserName, Secret = dockerHubRegistry?.PAT }, cancellationToken);
            var repositories = await dockerHub.GetRepositories(dockerHubRegistry.UserName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories?.Results?.Map(), null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<DockerHubTag>? tags, string? errorMessage)> GetRepositoryTagsAsync(DockerHubRegistry dockerHubRegistry, string repositoryName, CancellationToken cancellationToken)
    {
        try
        {
            var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = dockerHubRegistry.UserName, Secret = dockerHubRegistry.PAT }, cancellationToken);
            var repositories = await dockerHub.TagsGET(dockerHubRegistry.UserName, repositoryName, authResponse.Access_token, 1, 100, cancellationToken: cancellationToken);
            return (repositories.Results.Map(), null);
        }
        catch (ApiException ex)
        {
            return (null,
                ex.StatusCode == System.Net.HttpStatusCode.Unauthorized
                    ? "401 invalid DockerHub credentials, please check your PAT and/or your user-name."
                    : ex.Message);
        }
    }

    public async Task<(IEnumerable<DockerHubImageResult>? images, string? errorMessage)> SearchImage(string imageName, CancellationToken cancellationToken)
    {
        try
        {
            var pagedResult = await dockerHub.SearchImage(imageName, cancellationToken);
            return (pagedResult.Results?.Map().OrderByDescending(s => s.StarCount).ThenByDescending(s => s.PullCount), null);
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
