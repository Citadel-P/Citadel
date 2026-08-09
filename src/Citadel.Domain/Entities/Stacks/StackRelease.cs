using Domain.Entities.Platforms;
using Domain.Contracts.Resources.ResourceBindings;

namespace Domain.Entities.Stacks;

public sealed class StackRelease : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid StackId { get; private set; }
    public Guid PlatformId { get; private set; }
    public StackReleaseStatus Status { get; private set; } = StackReleaseStatus.Created;
    public string Version { get; private set; } = string.Empty;
    public StackSpec Spec { get; private set; } = null!;
    public StackReleaseSource? Source { get; private set; }
    public IReadOnlyList<ResourceBindingSnapshot>? ResourceBindings { get; private set; }

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; }
    #endregion

    public Platform? Platform { get; private set; } = null;
    public Domain.Entities.Identity.Actor? Actor { get; private set; } = null;
    public IReadOnlyList<Image>? Images { get; private set; } = null;
    public IReadOnlyList<Container>? Containers { get; private set; } = null;

    public static StackRelease Create(
        Guid stackId,
        Guid platformId,
        StackSpec spec,
        Guid createdByActorId,
        string? version,
        Platform? platform = null)
    {
        return new StackRelease
        {
            StackId = stackId,
            PlatformId = platformId,
            Version = version ?? "1",
            Status = StackReleaseStatus.Created,
            Spec = spec,
            CreatedByActorId = createdByActorId,
            Platform = platform,
        };
    }

    public static StackRelease FromPersistence(
        Guid id,
        Guid stackId,
        Guid platformId,
        StackReleaseStatus status,
        string version,
        StackSpec spec,
        StackReleaseSource? source,
        IReadOnlyList<ResourceBindingSnapshot>? resourceBindings,
        DateTime createdAt,
        Guid createdByActorId,
        Platform? platform = null,
        Domain.Entities.Identity.Actor? actor = null,
        IReadOnlyList<Image>? images = null,
        IReadOnlyList<Container>? containers = null)
    {
        return new StackRelease
        {
            Id = id,
            StackId = stackId,
            PlatformId = platformId,
            Status = status,
            Version = version,
            Spec = spec,
            Source = source,
            ResourceBindings = resourceBindings,
            CreatedAt = createdAt,
            CreatedByActorId = createdByActorId,
            Platform = platform,
            Actor = actor,
            Images = images,
            Containers = containers
        };
    }

    public void UpdateStackStatus(StackReleaseStatus status)
    {
        Status = status;
    }

    public void UpdateSpec(StackSpec spec)
    {
        Spec = spec;
    }

    public void UpdateDefinition(Guid platformId, StackSpec spec, Platform? platform = null)
    {
        PlatformId = platformId;
        Spec = spec;
        if (platform is not null)
            Platform = platform;
    }

    public void UpdateSource(StackReleaseSource source)
    {
        Source = source;
    }

    public void UpdateResourceBindings(IReadOnlyList<ResourceBindingSnapshot>? resourceBindings)
    {
        ResourceBindings = resourceBindings;
    }

    public StackRelease CreateSnapshot()
        => new()
        {
            StackId = StackId,
            PlatformId = PlatformId,
            Status = Status,
            Version = Version,
            Spec = Spec,
            Source = Source,
            ResourceBindings = ResourceBindings,
            CreatedAt = CreatedAt,
            CreatedByActorId = CreatedByActorId,
        };

    public bool IsRollbackCandidate()
        => Status == StackReleaseStatus.Healthy;

    public static string GetNextVersion(string currentVersion)
    {
        if (int.TryParse(currentVersion, out var currentVersionNumber))
        {
            return (currentVersionNumber + 1).ToString();
        }

        return $"{currentVersion}.1";
    }
}

public sealed record StackReleaseSource(
    StackSource SourceType,
    Guid? GitRepositoryId,
    string? GitRepositoryName,
    string? Branch,
    string? RequestedCommitSha,
    string ResolvedCommitSha,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths,
    string? GitRepositoryUrl = null,
    string? WorkingDirectory = null,
    IReadOnlyList<string>? WatchPaths = null,
    IReadOnlyList<string>? ComposeEnvFilesFromRepo = null,
    string? ComposeDigest = null);
