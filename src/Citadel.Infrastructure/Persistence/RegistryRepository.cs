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

    public Task<int> AddAsync(Registry registry, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Registries (
                Id, Name, Description, RegistryHost, Status, CreatedAt, CreatedByActorId, Configuration)
            VALUES (
                @Id, @Name, @Description, @RegistryHost, @Status, @CreatedAt, @CreatedByActorId, @Configuration::json)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id,
            Name = registry.Name,
            Description = registry.Description,
            RegistryHost = registry.RegistryHost, 
            CreatedAt = registry.CreatedAt, 
            CreatedByActorId = registry.CreatedByActorId,
            Status = EnumFormatter<RegistryStatus>.GetValue(registry.Status),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfiguration),
        }, transaction: tx());
    }

    public async Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Registry>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries r WHERE r.Id = ANY(@Ids)";
        var idArray = ids as Guid[] ?? [.. ids];
        var result = await db.QueryAsync<RegistryDto>(sql, new { Ids = idArray, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(sql, new { Name = name, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Registries WHERE Name=@Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id, cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries";
        var result = await db.QueryAsync<RegistryDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Registry>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT * FROM Registries r WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "r.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " ORDER BY r.CreatedAt DESC;";

        var grantedPermissionLevels = UserRepository.GetGrantedPermissionLevelValues(permissionLevel);
        var result = await db.QueryAsync<RegistryDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionLevels = grantedPermissionLevels,
            SpecificPermission = (int)specificPermission,
            cancellationToken
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
