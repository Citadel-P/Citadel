using Domain.Entities.Tags;

namespace WebApi.Routes.Endpoints.Resources.Tags;

public sealed record TagView(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    int UsageCount)
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
            tag.UsageCount);

    internal static TagView Map(Tag tag)
        => new(
            tag.Id,
            tag.Name,
            tag.NormalizedName,
            tag.Color,
            tag.CreatedByActorId,
            tag.CreatedAt,
            tag.UpdatedAt,
            0);
}

public sealed record TagsView(IReadOnlyList<TagView> Tags)
{
    internal static TagsView Map(IReadOnlyList<TagWithUsage> tags)
        => new([.. tags.Select(TagView.Map)]);
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
