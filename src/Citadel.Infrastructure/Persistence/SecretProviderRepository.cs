using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class SecretProviderRepository(IDbConnection db, Func<IDbTransaction> tx) : ISecretProviderRepository
{
    public Task<int> AddAsync(SecretProvider provider, CancellationToken cancellationToken)
    {
        provider.Validate();

        const string sql = """
            INSERT INTO SecretProviders (
                Id, Name, ProviderType, Configuration, CreatedAt, UpdatedAt)
            VALUES (
                @Id, @Name, @ProviderType, @Configuration, @CreatedAt, @UpdatedAt)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                provider.Id,
                provider.Name,
                ProviderType = EnumFormatter<SecretProviderType>.GetValue(provider.ProviderType),
                Configuration = JsonSerializer.Serialize(
                    provider.Configuration,
                    ConfigurationJsonContext.Default.VaultKvV2SecretProviderConfiguration),
                provider.CreatedAt,
                provider.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(SecretProvider provider, CancellationToken cancellationToken)
    {
        provider.Validate();

        const string sql = """
            UPDATE SecretProviders
            SET Name = @Name,
                ProviderType = @ProviderType,
                Configuration = @Configuration,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                provider.Id,
                provider.Name,
                ProviderType = EnumFormatter<SecretProviderType>.GetValue(provider.ProviderType),
                Configuration = JsonSerializer.Serialize(
                    provider.Configuration,
                    ConfigurationJsonContext.Default.VaultKvV2SecretProviderConfiguration),
                provider.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM SecretProviders WHERE Id = @Id";
        return db.ExecuteAsync(
            sql,
            new { Id = id },
            transaction: tx());
    }

    public async Task<SecretProvider?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM SecretProviders WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<SecretProviderDto>(
            sql,
            new { Id = id },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<IEnumerable<SecretProvider>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM SecretProviders ORDER BY Name ASC";
        var result = await db.QueryAsync<SecretProviderDto>(
            sql,
            transaction: tx());

        return result.Select(x => x.ToDomain());
    }

    public Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SecretProviders WHERE lower(Name) = lower(@Name))";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Name = name },
            transaction: tx());
    }

    public Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SecretProviders WHERE lower(Name) = lower(@Name) AND Id <> @Id)";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Name = name, Id = id },
            transaction: tx());
    }

    public Task<bool> IsUsedBySecretDefinitionAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SecretDefinitions WHERE ProviderId = @Id)";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Id = id },
            transaction: tx());
    }
}
