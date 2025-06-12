using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Registries;
using Infrastructure.DockerHub;
using Refit;

namespace Infrastructure.Services;

internal class DockerHubService(IDockerHubApi dockerHub) : IDockerHubService
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
}


internal static class DockerHubServiceMapper
{
    public static IEnumerable<DockerHubRepositoryInfo> Map(this IEnumerable<DockerHubRepository> dockerHubRepositories)
        => dockerHubRepositories.Select(Map);

    public static DockerHubRepositoryInfo Map(this DockerHubRepository dockerHubRepository)
        => new (
            Name: dockerHubRepository.Name,
            Namespace: dockerHubRepository.Namespace,
            IsPrivate: dockerHubRepository.IsPrivate,
            IsTrusted: dockerHubRepository.IsTrusted,
            LastUpdated: dockerHubRepository.LastUpdated,
            IsAutomated: dockerHubRepository.IsAutomated,
            PullCount: dockerHubRepository.PullCount);

    public static IEnumerable<DockerHubTag> Map(this ICollection<Tag> dockerHubTags)
        => dockerHubTags.Select(Map);

    public static DockerHubTag Map(this Tag dockerHubTag)
        => new (
            Id: dockerHubTag.Id,
            V2: dockerHubTag.V2,
            Name: dockerHubTag.Name,
            Creator: dockerHubTag.Creator,
            FullSize: dockerHubTag.Full_size,
            Repository: dockerHubTag.Repository,
            LastUpdated: dockerHubTag.Last_updated,
            LastUpdater: dockerHubTag.Last_updater,
            TagLastPulled: dockerHubTag.Tag_last_pulled,
            TagLastPushed: dockerHubTag.Tag_last_pushed,
            Images: dockerHubTag.Images?.Map() ?? [],
            LastUpdaterUsername: dockerHubTag.Last_updater_username,
            Status: dockerHubTag.Status == TagStatus.Active ? Domain.DockerHubTagStatus.Active : Domain.DockerHubTagStatus.Inactive);

    public static IEnumerable<DockerHubImage> Map(this ICollection<Image> dockerHubImages) 
        => dockerHubImages.Select(Map);

    public static DockerHubImage Map(this Image dockerHubImage) => new (
            Architecture: dockerHubImage.Architecture,
            Digest: dockerHubImage.Digest,
            Os: dockerHubImage.Os,
            Size: dockerHubImage.Size,
            Status: dockerHubImage.Status == ImageStatus.Active ? Domain.DockerHubImageStatus.Active : Domain.DockerHubImageStatus.Inactive,
            LastPulled: dockerHubImage.Last_pulled);
}