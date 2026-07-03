using Application.Permissions;
using Domain;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Identity;
using Domain.Contracts.Resources.ResourceBindings;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StacksView(IEnumerable<StackView> Stacks, ResourceCapabilities Capabilities)
{
    internal static async Task<StacksView> Map(IEnumerable<Stack> stacks, IPermissionEvaluator permissionEvaluator)
    {
        var list = stacks as Stack[] ?? [.. stacks];

        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Stack);
        if (list.Length == 0)
            return new StacksView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Stack);

        var views = new StackView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var stack = list[i];

            var baseView = StackView.Map(stack);

            perms.TryGetValue(stack.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToStackCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new StacksView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

public sealed record StackView(
    Guid Id,
    string Name,
    string? Description,
    StackSource StackSource,
    StackUpdateState StackUpdateState,
    StackDriftPolicy DriftPolicy,
    StackReleaseStatus Status,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    ResourceControlState ControlState,
    Guid CurrentStackReleaseId,
    Guid? PlatformId = null,
    string? Version = null,
    StackSpec? Spec = null,
    StackReleaseSource? Source = null,
    IReadOnlyList<ResourceBindingSnapshot>? ResourceBindings = null,
    PlatformStatus PlatformStatus = PlatformStatus.Offline,
    string? PlatformName = null,
    IReadOnlyList<TagSummaryView> Tags = null!,
    LatestActivityView? LatestActivityView = null,
    StackCapabilities? Capabilities = null)
{
    internal static StackView Map(Stack stack) => new(
        Id: stack.Id,
        Name: stack.Name,
        Description: stack.Description,
        StackSource: stack.StackSource,
        StackUpdateState: stack.StackUpdateState,
        DriftPolicy: stack.DriftPolicy,
        CreatedAt: stack.CreatedAt,
        CreatedByActorId: stack.CreatedByActorId,
        ControlState: stack.ControlState,
        CurrentStackReleaseId: stack.CurrentStackReleaseId,
        PlatformId: stack.CurrentStackRelease?.PlatformId,
        Status: stack.CurrentStackRelease?.Status != null ? stack.CurrentStackRelease.Status : StackReleaseStatus.Unknown,
        Version: stack.CurrentStackRelease?.Version,
        Spec: stack.CurrentStackRelease?.Spec,
        Source: stack.CurrentStackRelease?.Source,
        ResourceBindings: stack.CurrentStackRelease?.ResourceBindings,
        PlatformStatus: stack.CurrentStackRelease?.Platform?.Status ?? PlatformStatus.Offline,
        PlatformName: stack.CurrentStackRelease?.Platform?.Name,
        Tags: [.. stack.Tags.Select(TagSummaryView.Map)],
        LatestActivityView: stack.LatestActivityEvent?.Map());

    internal static async Task<StackView> Map(Stack stack, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(stack.Id, ResourceType.Stack);
        return Map(stack) with
        {
            Capabilities = CapabilityMapper.ToStackCapabilities(permissions)
        };
    }
}

public sealed record StackConfigView(
    Guid Id,
    string Name,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    StackUpdateState StackUpdateState,
    StackDriftPolicy DriftPolicy)
{
    internal static StackConfigView Map(Stack stack) => new(
        Id: stack.Id,
        Name: stack.Name,
        PlatformId: stack.CurrentStackRelease?.PlatformId ?? Guid.Empty,
        Description: stack.Description,
        StackSource: stack.StackSource,
        Spec: stack.CurrentStackRelease?.Spec!,
        StackUpdateState: stack.StackUpdateState,
        DriftPolicy: stack.DriftPolicy);
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
    StackReleaseSource? Source,
    IReadOnlyList<ResourceBindingSnapshot>? ResourceBindings,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string ActorName,
    ActorType ActorType,
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
        Source: release.Source,
        ResourceBindings: release.ResourceBindings,
        CreatedAt: release.CreatedAt,
        CreatedByActorId: release.CreatedByActorId,
        ActorName: release.Actor?.ActorMetadata?.Name ?? "Unknown",
        ActorType: release.Actor?.Type ?? ActorType.User,
        PlatformStatus: release.Platform?.Status ?? PlatformStatus.Offline,
        PlatformName: release.Platform?.Name);
}
