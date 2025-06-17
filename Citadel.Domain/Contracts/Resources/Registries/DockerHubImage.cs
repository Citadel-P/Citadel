namespace Domain.Contracts.Resources.Registries;

public sealed record DockerHubImage(
    string Architecture, 
    string Digest,
    string Os,
    int Size, 
    DockerHubImageStatus Status, 
    string LastPulled);

public sealed record DockerHubImageResult(
    string? Name,
    string? Description,
    bool IsOfficial,
    long StarCount,
    long PullCount,
    string? Url = null,
    string? Icon = null);