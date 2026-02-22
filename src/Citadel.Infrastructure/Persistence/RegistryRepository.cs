using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
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
                @Id, @Name, @Description, @RegistryHost, @Status, @CreatedAt, @CreatedByActorId, @Configuration)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id.Format(),
            Name = registry.Name,
            Description = registry.Description,
            RegistryHost = registry.RegistryHost, 
            CreatedAt = registry.CreatedAt.ToString(), 
            CreatedByActorId = registry.CreatedByActorId.Format(),
            Status = EnumFormatter<RegistryStatus>.GetValue(registry.Status),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfiguration),
        }, transaction: tx());
    }

    public async Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
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
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries";
        var result = await db.QueryAsync<RegistryDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Registries
            SET Name = @Name, Description = @Description, Status = @Status, RegistryHost = @RegistryHost, Configuration = @Configuration
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id.Format(),
            Name = registry.Name,
            Description = registry.Description,
            RegistryHost = registry.RegistryHost,
            Status = EnumFormatter<RegistryStatus>.GetValue(registry.Status),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfiguration),
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Registries
        WHERE Id IN (
            SELECT value FROM json_each(@Ids)
        )
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx()
        );
    }
}
