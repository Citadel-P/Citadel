using Domain;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal static class ResourceTagSql
{
    internal const string InputTagsCte = """
        WITH input_tags AS (
            SELECT DISTINCT unnest(@TagIds) AS TagId
        ),
        missing_tags AS (
            SELECT it.TagId
            FROM input_tags it
            WHERE NOT EXISTS (SELECT 1 FROM Tags t WHERE t.Id = it.TagId)
        ),
        """;

    internal static string TagAggregate(string resourceAlias)
        => $$"""
            COALESCE((
                SELECT jsonb_agg(jsonb_build_object('id', t.Id, 'name', t.Name, 'color', t.Color) ORDER BY t.Name)
                FROM ResourceTags rt
                JOIN Tags t ON t.Id = rt.TagId
                WHERE rt.ResourceType = @TagResourceType
                  AND rt.ResourceId = {{resourceAlias}}.Id
            ), '[]'::jsonb)::text AS TagsJson
            """;

    internal static string FilterPredicate(string resourceAlias)
        => $"""
            (@TagIdsLength = 0 OR EXISTS (
                SELECT 1
                FROM ResourceTags rtf
                WHERE rtf.ResourceType = @TagResourceType
                  AND rtf.ResourceId = {resourceAlias}.Id
                  AND rtf.TagId = ANY(@TagIds)
            ))
            """;

    internal static string InsertTagsCte(string insertedResourceCte, string resourceAlias)
        => $"""
            inserted_tags AS (
                INSERT INTO ResourceTags (ResourceType, ResourceId, TagId, CreatedAt, CreatedByActorId)
                SELECT @TagResourceType, {resourceAlias}.Id, it.TagId, @CreatedAt, @TagCreatedByActorId
                FROM {insertedResourceCte} {resourceAlias}
                JOIN input_tags it ON TRUE
                RETURNING TagId
            )
            """;

    internal static string InsertResultSelect(string insertedResourceCte, params string[] countedCtes)
    {
        var affectedRowsExpression = string.Join(" + ", countedCtes.Select(cte => $"(SELECT COUNT(*) FROM {cte})"));

        return $$"""
            SELECT
                ({{affectedRowsExpression}})::int AS AffectedRows,
                COALESCE((
                    SELECT jsonb_agg(jsonb_build_object('id', t.Id, 'name', t.Name, 'color', t.Color) ORDER BY t.Name)
                    FROM Tags t
                    JOIN input_tags it ON it.TagId = t.Id
                    WHERE EXISTS (SELECT 1 FROM {{insertedResourceCte}})
                ), '[]'::jsonb)::text AS TagsJson
            """;
    }

    internal static Guid[] NormalizeTagIds(IReadOnlyCollection<Guid>? tagIds)
        => tagIds is null ? [] : [.. tagIds.Where(id => id != Guid.Empty).Distinct()];

    internal static string GetResourceTypeValue(TaggableResourceType resourceType)
        => EnumFormatter<TaggableResourceType>.GetValue(resourceType);
}

internal sealed record ResourceInsertWithTagsResult(int AffectedRows, string? TagsJson);
