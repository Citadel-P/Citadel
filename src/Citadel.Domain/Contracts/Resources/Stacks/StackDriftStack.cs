using Domain;
using Domain.Entities.Stacks;

namespace Domain.Contracts.Resources.Stacks;

public sealed record StackDriftStack(
    Guid Id,
    Guid CurrentStackReleaseId,
    string Name,
    StackSource StackSource,
    Guid PlatformId,
    string? PlatformName,
    StackReleaseStatus Status,
    ResourceControlState ControlState,
    StackSpec Spec,
    StackDriftPolicy DriftPolicy);
