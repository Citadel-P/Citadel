namespace Domain.Contracts.Resources.Registries;

public sealed record DockerHubImage(
    string Architecture, 
    string Digest,
    string Os,
    int Size, 
    DockerHubImageStatus Status, 
    string LastPulled);