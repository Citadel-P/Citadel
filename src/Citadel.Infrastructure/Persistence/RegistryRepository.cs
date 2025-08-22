using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
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
                Id, Name, Url, Created, Type, Configuration)
            VALUES (
                @Id, @Name, @Url, @Created, @Type, @Configuration)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id.Format(),
            Name = registry.Name, 
            Url = registry.Url, 
            Created = registry.Created.ToString(), 
            Type = EnumFormatter<RegistryType>.GetValue(registry.Type),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfigurationBase),
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

    public Task<bool> IsNameUsedByAnotherRegistryAsync(Guid id, string name, CancellationToken cancellationToken)
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
            SET Name = @Name, Url = @Url, Type = @Type, Configuration = @Configuration
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = registry.Id.Format(),
            Name = registry.Name,
            Url = registry.Url,
            Created = registry.Created.ToString(),
            Type = EnumFormatter<RegistryType>.GetValue(registry.Type),
            Configuration = JsonSerializer.Serialize(registry.Configuration, RegistryJsonContext.Default.RegistryConfigurationBase),
        }, transaction: tx());
    }

    [DapperAot(false)]
    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForGuids("Id", ids);
        string sql = $"DELETE FROM Registries WHERE Id IN ({clause})";
        return db.ExecuteAsync(sql, parameters, transaction: tx());
    }
}
