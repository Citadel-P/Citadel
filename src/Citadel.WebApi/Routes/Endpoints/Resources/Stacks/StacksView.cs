using Domain;
using Domain.Entities.Stacks;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StacksView(IEnumerable<StackView> Stacks)
{
    internal static StacksView Map(IEnumerable<Stack> stacks) => new(stacks.Select(StackView.Map));
}

public sealed record StackView(
    Guid Id,
    string Name,
    string? Description,
    StackSource StackSource,
    StackUpdateState StackUpdateState,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    ResourceControlState ControlState,
    Guid CurrentStackReleaseId,
    Guid? PlatformId = null,
    StackReleaseStatus? Status = null,
    string? Version = null,
    StackSpec? Spec = null,
    PlatformStatus PlatformStatus = PlatformStatus.Offline,
    string? PlatformName = null)
{
    internal static StackView Map(Stack stack) => new(
        Id: stack.Id,
        Name: stack.Name,
        Description: stack.Description,
        StackSource: stack.StackSource,
        StackUpdateState: stack.StackUpdateState,
        CreatedAt: stack.CreatedAt,
        CreatedByActorId: stack.CreatedByActorId,
        ControlState: stack.ControlState,
        CurrentStackReleaseId: stack.CurrentStackReleaseId,
        PlatformId: stack.CurrentStackRelease?.PlatformId,
        Status: stack.CurrentStackRelease?.Status,
        Version: stack.CurrentStackRelease?.Version,
        Spec: stack.CurrentStackRelease?.Spec,
        PlatformStatus: stack.CurrentStackRelease?.Platform?.Status ?? PlatformStatus.Offline,
        PlatformName: stack.CurrentStackRelease?.Platform?.Name);
}

public sealed record StackConfigView(
    Guid Id,
    string Name,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    StackUpdateState StackUpdateState)
{
    internal static StackConfigView Map(Stack stack) => new(
        Id: stack.Id,
        Name: stack.Name,
        PlatformId: stack.CurrentStackRelease?.PlatformId ?? Guid.Empty,
        Description: stack.Description,
        StackSource: stack.StackSource,
        Spec: stack.CurrentStackRelease?.Spec!,
        StackUpdateState: stack.StackUpdateState);
}

public sealed record StackReleasesView(IEnumerable<StackReleaseView> Releases)
{
    internal static StackReleasesView Map(IEnumerable<StackRelease> releases) => new(releases.Select(StackReleaseView.Map));
}

public sealed record StackReleaseView(
    Guid Id,
    Guid StackId,
    Guid PlatformId,
    StackReleaseStatus Status,
    string Version,
    StackSpec Spec,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    PlatformStatus PlatformStatus = PlatformStatus.Offline,
    string? PlatformName = null)
{
    internal static StackReleaseView Map(StackRelease release) => new(
        Id: release.Id,
        StackId: release.StackId,
        PlatformId: release.PlatformId,
        Status: release.Status,
        Version: release.Version,
        Spec: release.Spec,
        CreatedAt: release.CreatedAt,
        CreatedByActorId: release.CreatedByActorId,
        PlatformStatus: release.Platform?.Status ?? PlatformStatus.Offline,
        PlatformName: release.Platform?.Name);
}