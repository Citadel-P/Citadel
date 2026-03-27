using Domain.Entities.Platforms;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Stacks;

public sealed class StackRelease : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid StackId { get; private set; }
    public Guid PlatformId { get; private set; }
    public StackReleaseStatus Status { get; private set; } = StackReleaseStatus.Created;
    public string Version { get; private set; } = string.Empty;
    public StackSpec Spec { get; private set; } = null!;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; }
    #endregion

    public Platform? Platform { get; private set; } = null;
    public IReadOnlyList<Image>? Images { get; private set; } = null;
    public IReadOnlyList<Container>? Containers { get; private set; } = null;

    public static StackRelease Create(
        Guid stackId,
        Guid platformId,
        StackSpec spec,
        Guid createdByActorId,
        string? version)
    {
        return new StackRelease
        {
            StackId = stackId,
            PlatformId = platformId,
            Version = version ?? "1",
            Status = StackReleaseStatus.Created,
            Spec = spec,
            CreatedByActorId = createdByActorId,
        };
    }

    public static StackRelease FromPersistence(
        Guid id,
        Guid stackId,
        Guid platformId,
        StackReleaseStatus status,
        string version,
        StackSpec spec,
        DateTime createdAt,
        Guid createdByActorId,
        Platform? platform = null,
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
            CreatedAt = createdAt,
            CreatedByActorId = createdByActorId,
            Platform = platform,
            Images = images,
            Containers = containers
        };
    }

    public void UpdateStackStatus(StackReleaseStatus status)
    {
        Status = status;
    }

    public static string GetNextVersion(string currentVersion)
    {
        if (int.TryParse(currentVersion, out var currentVersionNumber))
        {
            return (currentVersionNumber + 1).ToString();
        }

        return $"{currentVersion}.1";
    }
}
