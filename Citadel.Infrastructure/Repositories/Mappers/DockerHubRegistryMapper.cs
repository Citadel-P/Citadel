using Domain.Contracts.Resources.Registries;
using Infrastructure.DockerHub;

namespace Infrastructure.Repositories.Mappers;

internal static class DockerHubRegistryMapper
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

    public static IEnumerable<DockerHubImageResult> Map(this IEnumerable<DockerHubImageModel> dockerHubImageModels)
        => dockerHubImageModels.Select(Map);
    public static DockerHubImageResult Map(this DockerHubImageModel dockerHubImageModel) => new (
        Name: dockerHubImageModel.Name,
        Description: dockerHubImageModel.Description,
        IsOfficial: dockerHubImageModel.IsOfficial,
        StarCount: dockerHubImageModel.StarCount,
        PullCount: dockerHubImageModel.PullCount,
        Url: dockerHubImageModel.IsOfficial ? $"https://hub.docker.com/_/{dockerHubImageModel.Name}" : $"https://hub.docker.com/r/{dockerHubImageModel.Name}"
        );
}
