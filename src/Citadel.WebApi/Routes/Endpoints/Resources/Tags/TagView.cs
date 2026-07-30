using Application.Permissions;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Tags;

public sealed record TagView(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    int UsageCount,
    ResourceCapabilities? Capabilities = null)
{
    internal static TagView Map(TagWithUsage tag)
        => new(
            tag.Id,
            tag.Name,
            tag.NormalizedName,
            tag.Color,
            tag.CreatedByActorId,
            tag.CreatedAt,
            tag.UpdatedAt,
            tag.UsageCount,
            null);

    internal static TagView Map(Tag tag)
        => new(
            tag.Id,
            tag.Name,
            tag.NormalizedName,
            tag.Color,
            tag.CreatedByActorId,
            tag.CreatedAt,
            tag.UpdatedAt,
            0,
            null);
}

public sealed record TagsView(IReadOnlyList<TagView> Tags, ResourceCapabilities Capabilities)
{
    internal static async Task<TagsView> Map(
        IReadOnlyList<TagWithUsage> tags,
        IPermissionEvaluator permissionEvaluator)
    {
        var collectionPermissions = await permissionEvaluator.EvaluateAsync(ResourceType.Tag);
        if (tags.Count == 0)
            return new([], CapabilityMapper.ToResourceCapabilities(collectionPermissions));

        var ids = tags.Select(static tag => tag.Id).ToArray();
        var permissions = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Tag);
        var views = new TagView[tags.Count];

        for (var i = 0; i < tags.Count; i++)
        {
            var tag = tags[i];
            permissions.TryGetValue(tag.Id, out var metadata);
            views[i] = TagView.Map(tag) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    metadata == default ? PermissionMetadata.Empty : metadata)
            };
        }

        return new(views, CapabilityMapper.ToResourceCapabilities(collectionPermissions));
    }
}

public sealed record TagSummaryView(Guid Id, string Name, string Color)
{
    internal static TagSummaryView Map(TagSummary tag)
        => new(tag.Id, tag.Name, tag.Color);
}

public sealed record ResourceTagsView(IReadOnlyList<TagSummaryView> Tags)
{
    internal static ResourceTagsView Map(IReadOnlyList<TagSummary> tags)
        => new([.. tags.Select(TagSummaryView.Map)]);
}

public sealed record ReplaceResourceTagsInput(IReadOnlyCollection<Guid>? TagIds);

public sealed record CreateTagInput(string Name, string Color);

public sealed record PatchTagInput(string? Name, string? Color);
