using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class RegistryRepository(IDbConnection db, IDbTransaction tx) : IRegistryRepository 
{
    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Registries WHERE name = @Name)";
        return db.ExecuteScalarAsync<bool>(
            new CommandDefinition(sql, new { Name = name }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> AddAsync(Registry registry, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Registries (
                Id, Name, Url, Created, Type, Configuration)
            VALUES (
                @Id, @Name, @Url, @Created, @Type, @Configuration)
        """;

        var command = new CommandDefinition(sql, registry, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(command);
    }

    public async Task<Registry?> GetAsync(Guid Id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<RegistryDto>(new CommandDefinition(sql, new { Id }, transaction: tx, cancellationToken: cancellationToken));
        return result?.ToDomain();
    }

    public Task<bool> IsNameUsedByAnotherRegistryAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Registries WHERE Name=@Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(
            new CommandDefinition(sql, new { Name = name, Id = id }, transaction: tx, cancellationToken: cancellationToken));
    }

    public async Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Registries";
        var result = await db.QueryAsync<RegistryDto>(new CommandDefinition(sql, transaction: tx, cancellationToken: cancellationToken));
        return result?.ToDomain() ?? [];
    }

    public Task<RegistryConfigurationBase?> GetRegistryConfigurationAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Configuration
            FROM Registries
            WHERE name = @Name
            LIMIT 1
        """;

        return db.QuerySingleOrDefaultAsync<RegistryConfigurationBase>(
            new CommandDefinition(sql, new { Name = name }, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Registries
            SET Name = @Name, Url = @Url, Type = @Type, Configuration = @Configuration
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(new CommandDefinition(sql, registry, transaction: tx, cancellationToken: cancellationToken));
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM RegistriesWHERE Id IN @Ids";
        return db.ExecuteAsync(new CommandDefinition(sql, new { Ids = ids }, transaction: tx, cancellationToken: cancellationToken));
    }
}
