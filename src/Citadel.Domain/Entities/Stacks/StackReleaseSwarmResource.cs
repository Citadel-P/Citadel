namespace Domain.Entities.Stacks;

public sealed class StackReleaseSwarmResource(
    Guid stackReleaseId,
    Guid platformId,
    StackReleaseSwarmResourceKind kind,
    string dockerResourceId,
    string dockerResourceName,
    string composeResourceName,
    IReadOnlyList<StackReleaseSwarmResourceMount>? mounts = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid StackReleaseId { get; private set; } = stackReleaseId;
    public Guid PlatformId { get; private set; } = platformId;
    public StackReleaseSwarmResourceKind Kind { get; private set; } = kind;
    public string DockerResourceId { get; private set; } = dockerResourceId;
    public string DockerResourceName { get; private set; } = dockerResourceName;
    public string ComposeResourceName { get; private set; } = composeResourceName;
    public IReadOnlyList<StackReleaseSwarmResourceMount> Mounts { get; private set; } = mounts ?? [];

    public static StackReleaseSwarmResource FromPersistence(
        Guid id,
        Guid stackReleaseId,
        Guid platformId,
        StackReleaseSwarmResourceKind kind,
        string dockerResourceId,
        string dockerResourceName,
        string composeResourceName,
        IReadOnlyList<StackReleaseSwarmResourceMount>? mounts)
        => new(stackReleaseId, platformId, kind, dockerResourceId, dockerResourceName, composeResourceName, mounts)
        {
            Id = id
        };

    public StackReleaseSwarmResource ForRelease(Guid releaseId)
        => new(
            releaseId,
            PlatformId,
            Kind,
            DockerResourceId,
            DockerResourceName,
            ComposeResourceName,
            Mounts);
}

public sealed record StackReleaseSwarmResourceMount(string ServiceName, string TargetName);
