using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class RegistryRepository(IDbConnection db, Func<IDbTransaction> tx) : IRegistryRepository 
{
    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Registries WHERE name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public async Task<int> AddAsync(Registry registry, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null)
    {
        string sql = ResourceTagSql.InputTagsCte + """
            inserted_registry AS (
                INSERT INTO Registries (
                    Id, Name, Description, RegistryHost, Status, CreatedAt, CreatedByActorId, Configuration)
                SELECT
                    @Id, @Name, @Description, @RegistryHost, @Status, @CreatedAt, @CreatedByActorId, @Configuration::json
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_registry", "r") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_registry", "inserted_registry", "inserted_tags");
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);

        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(sql, new
        {
            Id = registry.Id,
            Name = registry.Name,
            Description = registry.Description,
            RegistryHost = registry.RegistryHost, 
            CreatedAt = registry.CreatedAt, 
            CreatedByActorId = registry.CreatedByActorId,
            Status = EnumFormatter<RegistryStatus>.GetValue(registry.Status),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfiguration),
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry),
            TagIds = tagIdArray,
            TagCreatedByActorId = tagCreatedByActorId ?? registry.CreatedByActorId
        }, transaction: tx());

        registry.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public async Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT
                r.*,
                {{ResourceTagSql.TagAggregate("r")}}
            FROM Registries r
            WHERE r.Id = @Id
            LIMIT 1
        """;
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(sql, new
        {
            Id = id,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry)
        }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Registry>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT
                r.*,
                {{ResourceTagSql.TagAggregate("r")}}
            FROM Registries r
            WHERE r.Id = ANY(@Ids)
        """;
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<RegistryDto>(sql, new
        {
            Ids = idArray,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry)
        }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT
                r.*,
                {{ResourceTagSql.TagAggregate("r")}}
            FROM Registries r
            WHERE r.Name = @Name
            LIMIT 1
        """;
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(sql, new
        {
            Name = name,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry)
        }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Registries WHERE Name=@Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id, cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = $$"""
            SELECT
                r.*,
                {{ResourceTagSql.TagAggregate("r")}}
            FROM Registries r
            WHERE {{ResourceTagSql.FilterPredicate("r")}}
        """;
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<RegistryDto>(sql, new
        {
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Registry>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + $$"""
            SELECT
                r.*,
                {{ResourceTagSql.TagAggregate("r")}}
            FROM Registries r
            WHERE 
            """
            + AuthorizationSql.ResourcePredicatePrefix + "r.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " AND " + ResourceTagSql.FilterPredicate("r")
            + " ORDER BY r.CreatedAt DESC;";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<RegistryDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Registry),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Registries
            SET Name = @Name, Description = @Description, Status = @Status, RegistryHost = @RegistryHost, Configuration = @Configuration::json
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id,
            Name = registry.Name,
            Description = registry.Description,
            RegistryHost = registry.RegistryHost,
            Status = EnumFormatter<RegistryStatus>.GetValue(registry.Status),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfiguration),
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Registries WHERE Id = ANY(@Ids)";

        return db.ExecuteAsync(
            sql,
            new { Ids = ids.ToArray(), cancellationToken },
            transaction: tx()
        );
    }
}
