using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class ConfigurationEntryRepository(IDbConnection db, Func<IDbTransaction> tx) : IConfigurationEntryRepository
{
    public Task<int> AddAsync(ConfigurationEntry entry, CancellationToken cancellationToken)
    {
        entry.Validate();

        const string sql = """
            INSERT INTO ConfigurationEntries (
                Id, Name, Kind, Scope, ResourceId, Value, SecretId, SecretDeliveryMode, TargetPath, CreatedAt, UpdatedAt)
            VALUES (
                @Id, @Name, @Kind, @Scope, @ResourceId, @Value, @SecretId, @SecretDeliveryMode, @TargetPath, @CreatedAt, @UpdatedAt)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = entry.Id,
                Name = entry.Name,
                Kind = EnumFormatter<ConfigurationEntryKind>.GetValue(entry.Kind),
                Scope = EnumFormatter<ConfigurationScope>.GetValue(entry.Scope),
                ResourceId = entry.ResourceId,
                Value = entry.Value,
                SecretId = entry.SecretId,
                SecretDeliveryMode = entry.SecretDeliveryMode is null
                    ? null
                    : EnumFormatter<SecretDeliveryMode>.GetValue(entry.SecretDeliveryMode.Value),
                TargetPath = entry.TargetPath,
                CreatedAt = entry.CreatedAt,
                UpdatedAt = entry.UpdatedAt
            },
            transaction: tx());
    }

    public async Task<int> ReplaceResourceEntriesAsync(ConfigurationScope scope, Guid resourceId, IEnumerable<ConfigurationEntry> entries, CancellationToken cancellationToken)
        => await ReplaceEntriesAsync(scope, resourceId, entries, cancellationToken);

    public async Task<int> ReplaceEntriesAsync(ConfigurationScope scope, Guid? resourceId, IEnumerable<ConfigurationEntry> entries, CancellationToken cancellationToken)
    {
        if (scope == ConfigurationScope.Global && resourceId is not null)
            throw new ArgumentException("Global entries cannot have a resource id.", nameof(resourceId));

        if (scope != ConfigurationScope.Global && resourceId is null)
            throw new ArgumentException("Resource scoped entries require a resource id.", nameof(resourceId));

        var entryArray = entries as ConfigurationEntry[] ?? [.. entries];
        foreach (var entry in entryArray)
        {
            entry.Validate();
            if (entry.Scope != scope || entry.ResourceId != resourceId)
                throw new ArgumentException("All entries must match the requested scope.");
        }

        var scopeValue = EnumFormatter<ConfigurationScope>.GetValue(scope);

        const string deleteSql = """
            DELETE FROM ConfigurationEntries
            WHERE Scope = @Scope
              AND ((@ResourceId IS NULL AND ResourceId IS NULL) OR ResourceId = @ResourceId)
        """;

        if (entryArray.Length == 0)
        {
            return await db.ExecuteAsync(
                deleteSql,
                new
                {
                    Scope = scopeValue,
                    ResourceId = resourceId
                },
                transaction: tx());
        }

        const string replaceSql = """
            WITH deleted AS (
                DELETE FROM ConfigurationEntries
                WHERE Scope = @Scope
                  AND ((@ResourceId IS NULL AND ResourceId IS NULL) OR ResourceId = @ResourceId)
                RETURNING 1
            ),
            inserted AS (
                INSERT INTO ConfigurationEntries (
                    Id, Name, Kind, Scope, ResourceId, Value, SecretId, SecretDeliveryMode, TargetPath, CreatedAt, UpdatedAt)
                SELECT
                    item.Id,
                    item.Name,
                    item.Kind,
                    item.Scope,
                    item.ResourceId,
                    item.Value,
                    item.SecretId,
                    item.SecretDeliveryMode,
                    item.TargetPath,
                    item.CreatedAt,
                    item.UpdatedAt
                FROM unnest(
                    @Ids::uuid[],
                    @Names::text[],
                    @Kinds::text[],
                    @Scopes::text[],
                    @ResourceIds::uuid[],
                    @Values::text[],
                    @SecretIds::uuid[],
                    @SecretDeliveryModes::text[],
                    @TargetPaths::text[],
                    @CreatedAts::timestamptz[],
                    @UpdatedAts::timestamptz[]
                ) AS item(
                    Id,
                    Name,
                    Kind,
                    Scope,
                    ResourceId,
                    Value,
                    SecretId,
                    SecretDeliveryMode,
                    TargetPath,
                    CreatedAt,
                    UpdatedAt)
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
                Scope = scopeValue,
                ResourceId = resourceId,
                Ids = entryArray.Select(x => x.Id).ToArray(),
                Names = entryArray.Select(x => x.Name).ToArray(),
                Kinds = entryArray.Select(x => EnumFormatter<ConfigurationEntryKind>.GetValue(x.Kind)).ToArray(),
                Scopes = entryArray.Select(x => EnumFormatter<ConfigurationScope>.GetValue(x.Scope)).ToArray(),
                ResourceIds = entryArray.Select(x => x.ResourceId).ToArray(),
                Values = entryArray.Select(x => x.Value).ToArray(),
                SecretIds = entryArray.Select(x => x.SecretId).ToArray(),
                SecretDeliveryModes = entryArray
                    .Select(x => x.SecretDeliveryMode is null
                        ? null
                        : EnumFormatter<SecretDeliveryMode>.GetValue(x.SecretDeliveryMode.Value))
                    .ToArray(),
                TargetPaths = entryArray.Select(x => x.TargetPath).ToArray(),
                CreatedAts = entryArray.Select(x => x.CreatedAt).ToArray(),
                UpdatedAts = entryArray.Select(x => x.UpdatedAt).ToArray()
            },
            transaction: tx());
    }

    public async Task<IEnumerable<ConfigurationEntry>> GetEntriesAsync(ConfigurationScope scope, Guid? resourceId, CancellationToken cancellationToken)
    {
        if (scope == ConfigurationScope.Global && resourceId is not null)
            throw new ArgumentException("Global entries cannot have a resource id.", nameof(resourceId));

        if (scope != ConfigurationScope.Global && resourceId is null)
            throw new ArgumentException("Resource scoped entries require a resource id.", nameof(resourceId));

        const string sql = """
            SELECT * FROM ConfigurationEntries
            WHERE Scope = @Scope
              AND ((@ResourceId IS NULL AND ResourceId IS NULL) OR ResourceId = @ResourceId)
            ORDER BY Name ASC
        """;

        var result = await db.QueryAsync<ConfigurationEntryDto>(
            sql,
            new
            {
                Scope = EnumFormatter<ConfigurationScope>.GetValue(scope),
                ResourceId = resourceId
            },
            transaction: tx());

        return result.Select(x => x.ToDomain());
    }

    public async Task<IEnumerable<ConfigurationEntry>> GetEffectiveEntriesAsync(ConfigurationScope scope, Guid resourceId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM ConfigurationEntries
            WHERE Scope = @GlobalScope
               OR (Scope = @Scope AND ResourceId = @ResourceId)
            ORDER BY Name ASC, Scope ASC
        """;

        var result = await db.QueryAsync<ConfigurationEntryDto>(
            sql,
            new
            {
                GlobalScope = EnumFormatter<ConfigurationScope>.GetValue(ConfigurationScope.Global),
                Scope = EnumFormatter<ConfigurationScope>.GetValue(scope),
                ResourceId = resourceId
            },
            transaction: tx());

        return result.Select(x => x.ToDomain());
    }

}
