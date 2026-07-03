using System.Text.RegularExpressions;

namespace Domain.Entities.Tags;

public sealed class Tag
{
    private Tag(string name, string color, Guid createdByActorId)
    {
        Name = name;
        NormalizedName = TagValidation.NormalizeName(name);
        Color = color;
        CreatedByActorId = createdByActorId;
    }

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; }
    public string NormalizedName { get; private set; }
    public string Color { get; private set; }
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; private set; } = DateTime.UtcNow;

    public Tag RenameAndRecolor(string name, string color)
    {
        Name = TagValidation.NormalizeDisplayName(name);
        NormalizedName = TagValidation.NormalizeName(name);
        Color = TagValidation.NormalizeColor(color);
        UpdatedAt = DateTime.UtcNow;

        Validate();
        return this;
    }

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("Tag name is required.", nameof(Name));

        if (Name.Trim().Length > TagValidation.MaxNameLength)
            throw new ArgumentException($"Tag name cannot exceed {TagValidation.MaxNameLength} characters.", nameof(Name));

        if (string.IsNullOrWhiteSpace(NormalizedName))
            throw new ArgumentException("Tag normalized name is required.", nameof(NormalizedName));

        if (!TagValidation.IsValidColor(Color))
            throw new ArgumentException("Tag color must be a valid hex color in #RRGGBB format.", nameof(Color));
    }

    public static Tag Create(string name, string color, Guid createdByActorId)
    {
        var tag = new Tag(
            TagValidation.NormalizeDisplayName(name),
            TagValidation.NormalizeColor(color),
            createdByActorId);

        tag.Validate();
        return tag;
    }

    public static Tag FromPersistence(
        Guid id,
        string name,
        string normalizedName,
        string color,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt)
    {
        return new Tag(name, color, createdByActorId)
        {
            Id = id,
            NormalizedName = normalizedName,
            CreatedAt = createdAt,
            UpdatedAt = updatedAt
        };
    }
}

public sealed class ResourceTag(
    TaggableResourceType resourceType,
    Guid resourceId,
    Guid tagId,
    Guid createdByActorId)
{
    public TaggableResourceType ResourceType { get; private set; } = resourceType;
    public Guid ResourceId { get; private set; } = resourceId;
    public Guid TagId { get; private set; } = tagId;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
}

public sealed record TagWithUsage(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    int UsageCount);

public sealed record TagSummary(Guid Id, string Name, string Color);

public static partial class TagValidation
{
    public const int MaxNameLength = 64;

    public static string NormalizeDisplayName(string name)
        => name.Trim();

    public static string NormalizeName(string name)
        => name.Trim().ToLowerInvariant();

    public static string NormalizeColor(string color)
        => color.Trim().ToUpperInvariant();

    public static bool IsValidColor(string? color)
        => !string.IsNullOrWhiteSpace(color) && HexColorRegex().IsMatch(color.Trim());

    [GeneratedRegex("^#[0-9A-Fa-f]{6}$", RegexOptions.Compiled)]
    private static partial Regex HexColorRegex();
}
