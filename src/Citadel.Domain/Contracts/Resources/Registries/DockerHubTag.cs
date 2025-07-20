namespace Domain.Contracts.Resources.Registries;

public sealed record DockerHubTag(
    int Id,
    bool V2,
    int Creator,
    string Name,
    int FullSize,
    int Repository,
    int LastUpdater,
    string LastUpdated,
    string TagLastPulled,
    string TagLastPushed,
    DockerHubTagStatus Status,
    string LastUpdaterUsername,
    IEnumerable<DockerHubImage> Images);
