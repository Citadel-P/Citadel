using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class TagRepository(IDbConnection db, Func<IDbTransaction> tx) : ITagRepository
{
    public async Task<IReadOnlyList<TagWithUsage>> ListAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                t.Id,
                t.Name,
                t.NormalizedName,
                t.Color,
                t.CreatedByActorId,
                t.CreatedAt,
                t.UpdatedAt,
                COUNT(rt.TagId)::int AS UsageCount
            FROM Tags t
            LEFT JOIN ResourceTags rt ON rt.TagId = t.Id
            GROUP BY t.Id, t.Name, t.NormalizedName, t.Color, t.CreatedByActorId, t.CreatedAt, t.UpdatedAt
            ORDER BY t.Name ASC
        """;

        var result = await db.QueryAsync<TagWithUsageDto>(sql, transaction: tx());
        return [.. result.Select(x => x.ToDomain())];
    }

    public async Task<Tag?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Tags WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<TagDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Tag?> GetByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Tags WHERE NormalizedName = @NormalizedName LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<TagDto>(
            sql,
            new { NormalizedName = normalizedName },
            transaction: tx());

        return result?.ToDomain();
    }

    public Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Tags WHERE NormalizedName = @NormalizedName)";
        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName }, transaction: tx());
    }

    public Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Tags WHERE NormalizedName = @NormalizedName AND Id <> @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName, Id = id }, transaction: tx());
    }

    public Task<int> AddAsync(Tag tag, CancellationToken cancellationToken)
    {
        tag.Validate();

        const string sql = """
            INSERT INTO Tags (Id, Name, NormalizedName, Color, CreatedByActorId, CreatedAt, UpdatedAt)
            VALUES (@Id, @Name, @NormalizedName, @Color, @CreatedByActorId, @CreatedAt, @UpdatedAt)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                tag.Id,
                tag.Name,
                tag.NormalizedName,
                tag.Color,
                tag.CreatedByActorId,
                tag.CreatedAt,
                tag.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(Tag tag, CancellationToken cancellationToken)
    {
        tag.Validate();

        const string sql = """
            UPDATE Tags
            SET Name = @Name,
                NormalizedName = @NormalizedName,
                Color = @Color,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                tag.Id,
                tag.Name,
                tag.NormalizedName,
                tag.Color,
                tag.UpdatedAt
            },
            transaction: tx());
    }

    public async Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string bindingsSql = "DELETE FROM ResourceTags WHERE TagId = @Id";
        var deletedBindings = await db.ExecuteAsync(bindingsSql, new { Id = id }, transaction: tx());

        const string tagSql = "DELETE FROM Tags WHERE Id = @Id";
        var deletedTags = await db.ExecuteAsync(tagSql, new { Id = id }, transaction: tx());

        return deletedBindings + deletedTags;
    }
}

internal sealed class ResourceTagRepository(IDbConnection db, Func<IDbTransaction> tx) : IResourceTagRepository
{
    public async Task<IReadOnlyList<TagSummary>> GetForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        CancellationToken cancellationToken)
    {
        var result = await GetForResourcesAsync(resourceType, [resourceId], cancellationToken);
        return result.TryGetValue(resourceId, out var tags) ? tags : [];
    }

    public async Task<IReadOnlyDictionary<Guid, IReadOnlyList<TagSummary>>> GetForResourcesAsync(
        TaggableResourceType resourceType,
        IReadOnlyCollection<Guid> resourceIds,
        CancellationToken cancellationToken)
    {
        if (resourceIds.Count == 0)
            return new Dictionary<Guid, IReadOnlyList<TagSummary>>();

        const string sql = """
            SELECT
                rt.ResourceId,
                t.Id,
                t.Name,
                t.Color
            FROM ResourceTags rt
            JOIN Tags t ON t.Id = rt.TagId
            WHERE rt.ResourceType = @ResourceType
              AND rt.ResourceId = ANY(@ResourceIds::uuid[])
            ORDER BY t.Name ASC
        """;

        var rows = await db.QueryAsync<TagSummaryDto>(
            sql,
            new
            {
                ResourceType = EnumFormatter<TaggableResourceType>.GetValue(resourceType),
                ResourceIds = resourceIds.ToArray()
            },
            transaction: tx());

        return rows
            .GroupBy(x => x.ResourceId)
            .ToDictionary(
                x => x.Key,
                x => (IReadOnlyList<TagSummary>)[.. x.Select(row => row.ToSummary())]);
    }

    public async Task<int> ReplaceForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid> tagIds,
        Guid createdByActorId,
        DateTime now,
        CancellationToken cancellationToken)
    {
        var distinctTagIds = tagIds.Distinct().ToArray();
        var resourceTypeValue = EnumFormatter<TaggableResourceType>.GetValue(resourceType);

        const string deleteSql = """
            DELETE FROM ResourceTags
            WHERE ResourceType = @ResourceType
              AND ResourceId = @ResourceId
        """;

        if (distinctTagIds.Length == 0)
        {
            return await db.ExecuteAsync(
                deleteSql,
                new { ResourceType = resourceTypeValue, ResourceId = resourceId },
                transaction: tx());
        }

        const string replaceSql = """
            WITH deleted AS (
                DELETE FROM ResourceTags
                WHERE ResourceType = @ResourceType
                  AND ResourceId = @ResourceId
                RETURNING 1
            ),
            inserted AS (
                INSERT INTO ResourceTags (ResourceType, ResourceId, TagId, CreatedAt, CreatedByActorId)
                SELECT
                    @ResourceType,
                    @ResourceId,
                    item.TagId,
                    @CreatedAt,
                    @CreatedByActorId
                FROM unnest(@TagIds::uuid[]) AS item(TagId)
                RETURNING 1
            )
            SELECT
                (SELECT COUNT(*) FROM deleted) +
                (SELECT COUNT(*) FROM inserted)
        """;

        return await db.QuerySingleAsync<int>(
            replaceSql,
            new
            {
                ResourceType = resourceTypeValue,
                ResourceId = resourceId,
                TagIds = distinctTagIds,
                CreatedAt = now,
                CreatedByActorId = createdByActorId
            },
            transaction: tx());
    }

    public async Task<bool> AllTagsExistAsync(IReadOnlyCollection<Guid> tagIds, CancellationToken cancellationToken)
    {
        var distinctTagIds = tagIds.Distinct().ToArray();
        if (distinctTagIds.Length == 0)
            return true;

        const string sql = "SELECT COUNT(*)::int FROM Tags WHERE Id = ANY(@TagIds::uuid[])";
        var count = await db.ExecuteScalarAsync<int>(sql, new { TagIds = distinctTagIds }, transaction: tx());
        return count == distinctTagIds.Length;
    }
}
