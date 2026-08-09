namespace Infrastructure.Persistence.Dtos;

internal sealed record StackReleaseSwarmResourceDto(
    Guid Id,
    Guid StackReleaseId,
    Guid PlatformId,
    string Kind,
    string DockerResourceId,
    string DockerResourceName,
    string ComposeResourceName,
    string Mounts);
