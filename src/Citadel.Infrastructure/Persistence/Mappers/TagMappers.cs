using Domain;
using Domain.Entities.Tags;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class TagMappers
{
    internal static Tag ToDomain(this TagDto dto)
        => Tag.FromPersistence(
            dto.Id,
            dto.Name,
            dto.NormalizedName,
            dto.Color,
            dto.CreatedByActorId,
            dto.CreatedAt,
            dto.UpdatedAt);

    internal static TagWithUsage ToDomain(this TagWithUsageDto dto)
        => new(
            dto.Id,
            dto.Name,
            dto.NormalizedName,
            dto.Color,
            dto.CreatedByActorId,
            dto.CreatedAt,
            dto.UpdatedAt,
            dto.UsageCount);

    internal static TagSummary ToSummary(this TagSummaryDto dto)
        => new(dto.Id, dto.Name, dto.Color);

    internal static IReadOnlyList<TagSummary> ToTagSummaries(this string? tagsJson)
        => string.IsNullOrWhiteSpace(tagsJson)
            ? []
            : JsonSerializer.Deserialize(tagsJson, TagJsonContext.Default.IReadOnlyListTagSummary) ?? [];
}
