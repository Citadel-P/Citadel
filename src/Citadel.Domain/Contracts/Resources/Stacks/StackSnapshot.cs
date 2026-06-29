using Domain.Entities.Stacks;

namespace Domain.Contracts.Resources.Stacks;

public sealed record StackSnapshot(
    Guid Id,
    string Name,
    string? Description,
    StackSource StackSource,
    StackDriftPolicy DriftPolicy,
    StackReleaseSnapshot? StackRelease);

public sealed record GitStackBranchSubscription(
    Guid StackId,
    Guid GitRepositoryId,
    string Branch);

public sealed record StackReleaseSnapshot(
    Guid PlatformId,
    StackSpec Spec,
    Guid CreatedByActorId,
    string? Version,
    StackReleaseSource? Source = null,
    IReadOnlyList<Domain.Contracts.Resources.Configuration.ConfigurationSnapshotEntry>? Configuration = null);

public static class StackSnapshotExtensions
{
    public static StackSnapshot ToSnapshot(this Stack stack, Guid? id = null)
        => new(
            Id: id ?? stack.Id,
            Name: stack.Name,
            Description: stack.Description,
            StackSource: stack.StackSource,
            DriftPolicy: stack.DriftPolicy,
            StackRelease: stack.CurrentStackRelease?.ToSnapshot());

    public static StackSnapshot ToSnapshot(this StackPatchModel stack, StackRelease stackRelease, Guid? id = null)
        => new(
            Id: id ?? Guid.NewGuid(),
            Name: stack.Name,
            Description: stack.Description,
            StackSource: stack.StackSource ?? StackSource.WebEditor,
            DriftPolicy: stack.DriftPolicy ?? StackDriftPolicy.Default,
            StackRelease: stackRelease?.ToSnapshot());

    public static StackReleaseSnapshot ToSnapshot(this StackRelease stackRelease)
        => new(
            PlatformId: stackRelease.PlatformId,
            Spec: stackRelease.Spec,
            CreatedByActorId: stackRelease.CreatedByActorId,
            Version: stackRelease.Version,
            Source: stackRelease.Source,
            Configuration: stackRelease.Configuration);
}

public sealed record StackResultSnapshot(
    IEnumerable<string>? ContainerIds = null,
    string? Message = null,
    IReadOnlyList<Domain.Contracts.Resources.Configuration.ConfigurationSnapshotEntry>? Configuration = null);
