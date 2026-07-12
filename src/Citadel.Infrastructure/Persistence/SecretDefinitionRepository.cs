using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class SecretDefinitionRepository(IDbConnection db, Func<IDbTransaction> tx) : ISecretDefinitionRepository
{
    public async Task<int> AddAsync(SecretDefinition secret, InternalSecretValue? value, CancellationToken cancellationToken)
    {
        secret.Validate();

        const string secretSql = """
            INSERT INTO SecretDefinitions (
                Id, Name, ProviderType, ProviderId, ExternalPath, ExternalKey, ExternalVersion, CreatedAt, UpdatedAt)
            VALUES (
                @Id, @Name, @ProviderType, @ProviderId, @ExternalPath, @ExternalKey, @ExternalVersion, @CreatedAt, @UpdatedAt)
        """;

        var count = await db.ExecuteAsync(
            secretSql,
            new
            {
                secret.Id,
                secret.Name,
                ProviderType = EnumFormatter<SecretProviderType>.GetValue(secret.ProviderType),
                secret.ProviderId,
                secret.ExternalPath,
                secret.ExternalKey,
                secret.ExternalVersion,
                secret.CreatedAt,
                secret.UpdatedAt
            },
            transaction: tx());

        if (value is null)
            return count;

        const string valueSql = """
            INSERT INTO InternalSecretValues (SecretId, EncryptedValue, CreatedAt, UpdatedAt)
            VALUES (@SecretId, @EncryptedValue, @CreatedAt, @UpdatedAt)
        """;

        return count + await db.ExecuteAsync(
            valueSql,
            new
            {
                value.SecretId,
                value.EncryptedValue,
                value.CreatedAt,
                value.UpdatedAt
            },
            transaction: tx());
    }

    public async Task<SecretDefinition?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM SecretDefinitions WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<SecretDefinitionDto>(
            sql,
            new { Id = id },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<int> UpdateAsync(SecretDefinition secret, CancellationToken cancellationToken)
    {
        secret.Validate();

        const string sql = """
            UPDATE SecretDefinitions
            SET Name = @Name,
                ProviderType = @ProviderType,
                ProviderId = @ProviderId,
                ExternalPath = @ExternalPath,
                ExternalKey = @ExternalKey,
                ExternalVersion = @ExternalVersion,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;

        return await db.ExecuteAsync(
            sql,
            new
            {
                secret.Id,
                secret.Name,
                ProviderType = EnumFormatter<SecretProviderType>.GetValue(secret.ProviderType),
                secret.ProviderId,
                secret.ExternalPath,
                secret.ExternalKey,
                secret.ExternalVersion,
                UpdatedAt = DateTime.UtcNow
            },
            transaction: tx());
    }

    public async Task<IEnumerable<SecretDefinition>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM SecretDefinitions ORDER BY Name ASC";
        var result = await db.QueryAsync<SecretDefinitionDto>(
            sql,
            transaction: tx());

        return result.Select(x => x.ToDomain());
    }

    public async Task<IEnumerable<SecretDefinition>> GetBoundAsync(ResourceBindingScope scope, Guid? resourceId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT DISTINCT sd.*
            FROM SecretDefinitions sd
            JOIN ResourceBindings rb ON rb.SecretId = sd.Id
            WHERE rb.Scope = @Scope
              AND ((@ResourceId IS NULL AND rb.ResourceId IS NULL) OR rb.ResourceId = @ResourceId)
            ORDER BY sd.Name ASC
        """;

        var result = await db.QueryAsync<SecretDefinitionDto>(
            sql,
            new
            {
                Scope = EnumFormatter<ResourceBindingScope>.GetValue(scope),
                ResourceId = resourceId
            },
            transaction: tx());

        return result.Select(x => x.ToDomain());
    }

    public async Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string valueSql = "DELETE FROM InternalSecretValues WHERE SecretId = @Id";
        var deletedValues = await db.ExecuteAsync(valueSql, new { Id = id }, transaction: tx());

        const string secretSql = "DELETE FROM SecretDefinitions WHERE Id = @Id";
        var deletedSecrets = await db.ExecuteAsync(secretSql, new { Id = id }, transaction: tx());

        return deletedValues + deletedSecrets;
    }

    public async Task<InternalSecretValue?> GetInternalValueAsync(Guid secretId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM InternalSecretValues WHERE SecretId = @SecretId LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<InternalSecretValueDto>(
            sql,
            new { SecretId = secretId },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<IReadOnlyList<SecretResolutionMaterial>> GetResolutionMaterialsAsync(
        IReadOnlyCollection<Guid> secretIds,
        CancellationToken cancellationToken)
    {
        var ids = secretIds.Where(static id => id != Guid.Empty).Distinct().ToArray();
        if (ids.Length == 0)
            return [];

        const string sql = """
            SELECT
                sd.Id,
                sd.Name,
                sd.ProviderType,
                sd.ProviderId,
                sd.ExternalPath,
                sd.ExternalKey,
                sd.ExternalVersion,
                sd.CreatedAt,
                sd.UpdatedAt,
                iv.EncryptedValue AS InternalEncryptedValue,
                iv.CreatedAt AS InternalValueCreatedAt,
                iv.UpdatedAt AS InternalValueUpdatedAt,
                sp.Id AS ExternalProviderId,
                sp.Name AS ExternalProviderName,
                sp.ProviderType AS ExternalProviderType,
                sp.Configuration AS ExternalProviderConfiguration,
                sp.CreatedAt AS ExternalProviderCreatedAt,
                sp.UpdatedAt AS ExternalProviderUpdatedAt
            FROM SecretDefinitions sd
            LEFT JOIN InternalSecretValues iv ON iv.SecretId = sd.Id
            LEFT JOIN SecretProviders sp ON sp.Id = sd.ProviderId
            WHERE sd.Id = ANY(@Ids)
            """;

        var rows = await db.QueryAsync<SecretResolutionMaterialDto>(
            sql,
            new { Ids = ids },
            transaction: tx());

        return [.. rows.Select(static x => x.ToDomain())];
    }

    public Task<int> DeleteExternalByProviderIdAsync(Guid providerId, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM SecretDefinitions
            WHERE ProviderId = @ProviderId
              AND ProviderType = @ProviderType
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ProviderId = providerId,
                ProviderType = EnumFormatter<SecretProviderType>.GetValue(SecretProviderType.VaultCompatibleKvV2)
            },
            transaction: tx());
    }

    public Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SecretDefinitions WHERE lower(Name) = lower(@Name))";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Name = name },
            transaction: tx());
    }

    public Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM SecretDefinitions WHERE lower(Name) = lower(@Name) AND Id <> @Id)";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Name = name, Id = id },
            transaction: tx());
    }

    public Task<bool> IsUsedByResourceBindingAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM ResourceBindings WHERE SecretId = @Id)";
        return db.ExecuteScalarAsync<bool>(
            sql,
            new { Id = id },
            transaction: tx());
    }
}
