using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using static Infrastructure.TypeHandlers.FormattingExtensions;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class BackupRepositoryRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRepositoryRepository
{
    public Task<int> AddAsync(BackupRepository repository, CancellationToken cancellationToken)
    {
        repository.Validate();

        const string sql = """
            INSERT INTO BackupRepositories (
                Id, Name, NormalizedName, Description, Type, Spec, PasswordSecretId, Status,
                LastPrunedAt, LastCheckedAt, CreatedByActorId, CreatedAt, UpdatedAt, ArchivedAt, RowVersion)
            VALUES (
                @Id, @Name, @NormalizedName, @Description, @Type, @Spec::jsonb, @PasswordSecretId, @Status,
                @LastPrunedAt, @LastCheckedAt, @CreatedByActorId, @CreatedAt, @UpdatedAt, @ArchivedAt, @RowVersion)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                repository.Id,
                repository.Name,
                repository.NormalizedName,
                repository.Description,
                Type = EnumFormatter<BackupRepositoryType>.GetValue(repository.Type),
                Spec = BackupMappers.SerializeRepositorySpec(repository.Spec),
                repository.PasswordSecretId,
                Status = EnumFormatter<BackupRepositoryStatus>.GetValue(repository.Status),
                LastPrunedAt = BackupMappers.ToUtcDateTime(repository.LastPrunedAt),
                LastCheckedAt = BackupMappers.ToUtcDateTime(repository.LastCheckedAt),
                repository.CreatedByActorId,
                CreatedAt = BackupMappers.ToUtcDateTime(repository.CreatedAt),
                UpdatedAt = BackupMappers.ToUtcDateTime(repository.UpdatedAt),
                ArchivedAt = BackupMappers.ToUtcDateTime(repository.ArchivedAt),
                repository.RowVersion
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(BackupRepository repository, CancellationToken cancellationToken)
    {
        repository.Validate();

        const string sql = """
            UPDATE BackupRepositories
            SET Name = @Name,
                NormalizedName = @NormalizedName,
                Description = @Description,
                Type = @Type,
                Spec = @Spec::jsonb,
                PasswordSecretId = @PasswordSecretId,
                Status = @Status,
                LastPrunedAt = @LastPrunedAt,
                LastCheckedAt = @LastCheckedAt,
                UpdatedAt = @UpdatedAt,
                ArchivedAt = @ArchivedAt,
                RowVersion = @RowVersion
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                repository.Id,
                repository.Name,
                repository.NormalizedName,
                repository.Description,
                Type = EnumFormatter<BackupRepositoryType>.GetValue(repository.Type),
                Spec = BackupMappers.SerializeRepositorySpec(repository.Spec),
                repository.PasswordSecretId,
                Status = EnumFormatter<BackupRepositoryStatus>.GetValue(repository.Status),
                LastPrunedAt = BackupMappers.ToUtcDateTime(repository.LastPrunedAt),
                LastCheckedAt = BackupMappers.ToUtcDateTime(repository.LastCheckedAt),
                UpdatedAt = BackupMappers.ToUtcDateTime(repository.UpdatedAt),
                ArchivedAt = BackupMappers.ToUtcDateTime(repository.ArchivedAt),
                repository.RowVersion
            },
            transaction: tx());
    }

    public Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRepositories
            SET ArchivedAt = COALESCE(ArchivedAt, @ArchivedAt),
                UpdatedAt = @ArchivedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(sql, new { Id = id, ArchivedAt = BackupMappers.ToUtcDateTime(archivedAt) }, transaction: tx());
    }

    public async Task<BackupRepositoryArchiveResult> ArchiveIfUnusedAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH target AS (
                SELECT Id
                FROM BackupRepositories
                WHERE Id = @Id
                  AND ArchivedAt IS NULL
            ),
            active_operations AS (
                SELECT 1
                FROM BackupRuns
                WHERE BackupRepositoryId = @Id
                  AND Status = ANY(@BackupStatuses)
                LIMIT 1
            ),
            active_restores AS (
                SELECT 1
                FROM BackupRestoreRuns
                WHERE BackupRepositoryId = @Id
                  AND Status = ANY(@RestoreStatuses)
                LIMIT 1
            ),
            active_policies AS (
                SELECT 1
                FROM BackupPolicies
                WHERE BackupRepositoryId = @Id
                  AND ArchivedAt IS NULL
                LIMIT 1
            ),
            archived AS (
                UPDATE BackupRepositories
                SET ArchivedAt = @ArchivedAt,
                    UpdatedAt = @ArchivedAt,
                    RowVersion = RowVersion + 1
                WHERE Id = @Id
                  AND ArchivedAt IS NULL
                  AND NOT EXISTS (SELECT 1 FROM active_operations)
                  AND NOT EXISTS (SELECT 1 FROM active_restores)
                  AND NOT EXISTS (SELECT 1 FROM active_policies)
                RETURNING 1
            )
            SELECT CASE
                WHEN EXISTS (SELECT 1 FROM archived) THEN 0
                WHEN NOT EXISTS (SELECT 1 FROM target) THEN 1
                WHEN EXISTS (SELECT 1 FROM active_operations) OR EXISTS (SELECT 1 FROM active_restores) THEN 2
                ELSE 3
            END
            """;

        var result = await db.ExecuteScalarAsync<int>(
            sql,
            new
            {
                Id = id,
                ArchivedAt = BackupMappers.ToUtcDateTime(archivedAt),
                BackupStatuses = ActiveBackupStatuses(),
                RestoreStatuses = ActiveRestoreStatuses()
            },
            transaction: tx());

        return result switch
        {
            0 => BackupRepositoryArchiveResult.Archived,
            1 => BackupRepositoryArchiveResult.NotFound,
            2 => BackupRepositoryArchiveResult.ActiveOperation,
            _ => BackupRepositoryArchiveResult.HasPolicies
        };
    }

    public Task<int> ApplyValidationResultAsync(
        BackupRepositoryValidation validation,
        bool markChecked,
        bool markPruned,
        CancellationToken cancellationToken)
    {
        validation.Validate();

        const string sql = """
            WITH updated_repository AS (
                UPDATE BackupRepositories
                SET Status = CASE
                        WHEN @Status = @ReadyStatus THEN @ReadyStatus
                        WHEN @Status = @UninitializedStatus THEN @UninitializedStatus
                        ELSE Status
                    END,
                    LastCheckedAt = CASE WHEN @MarkChecked THEN @LastValidatedAt ELSE LastCheckedAt END,
                    LastPrunedAt = CASE WHEN @MarkPruned THEN @LastValidatedAt ELSE LastPrunedAt END,
                    UpdatedAt = CASE
                        WHEN @Status IN (@ReadyStatus, @UninitializedStatus) OR @MarkChecked OR @MarkPruned
                            THEN @LastValidatedAt
                        ELSE UpdatedAt
                    END,
                    RowVersion = CASE
                        WHEN @Status IN (@ReadyStatus, @UninitializedStatus) OR @MarkChecked OR @MarkPruned
                            THEN RowVersion + 1
                        ELSE RowVersion
                    END
                WHERE Id = @BackupRepositoryId
                RETURNING 1
            ),
            updated_validation AS (
                UPDATE BackupRepositoryValidations
                SET Status = @Status,
                    LastValidatedAt = @LastValidatedAt,
                    LastErrorCode = @LastErrorCode,
                    LastErrorMessage = @LastErrorMessage
                WHERE BackupRepositoryId = @BackupRepositoryId
                  AND Location = @Location
                  AND ((PlatformId IS NULL AND @PlatformId IS NULL) OR PlatformId = @PlatformId)
                RETURNING 1
            ),
            inserted_validation AS (
                INSERT INTO BackupRepositoryValidations (
                    Id, BackupRepositoryId, Location, PlatformId, Status, LastValidatedAt, LastErrorCode, LastErrorMessage)
                SELECT @Id, @BackupRepositoryId, @Location, @PlatformId, @Status, @LastValidatedAt, @LastErrorCode, @LastErrorMessage
                WHERE EXISTS (SELECT 1 FROM updated_repository)
                  AND NOT EXISTS (SELECT 1 FROM updated_validation)
                RETURNING 1
            )
            SELECT COALESCE((SELECT COUNT(*) FROM updated_repository), 0)
                 + COALESCE((SELECT COUNT(*) FROM updated_validation), 0)
                 + COALESCE((SELECT COUNT(*) FROM inserted_validation), 0)
            """;

        return db.ExecuteScalarAsync<int>(
            sql,
            new
            {
                validation.Id,
                validation.BackupRepositoryId,
                Location = EnumFormatter<BackupExecutionLocation>.GetValue(validation.Location),
                validation.PlatformId,
                Status = EnumFormatter<BackupRepositoryValidationStatus>.GetValue(validation.Status),
                ReadyStatus = EnumFormatter<BackupRepositoryValidationStatus>.GetValue(BackupRepositoryValidationStatus.Ready),
                UninitializedStatus = EnumFormatter<BackupRepositoryValidationStatus>.GetValue(BackupRepositoryValidationStatus.Uninitialized),
                LastValidatedAt = BackupMappers.ToUtcDateTime(validation.LastValidatedAt),
                validation.LastErrorCode,
                validation.LastErrorMessage,
                MarkChecked = markChecked,
                MarkPruned = markPruned
            },
            transaction: tx());
    }

    public async Task<BackupRepository?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false)
    {
        const string sql = """
            SELECT *
            FROM BackupRepositories
            WHERE Id = @Id
              AND (@IncludeArchived OR ArchivedAt IS NULL)
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRepositoryDto>(
            sql,
            new { Id = id, IncludeArchived = includeArchived },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BackupRepository>> GetAllAsync(CancellationToken cancellationToken, bool includeArchived = false)
    {
        const string sql = """
            SELECT *
            FROM BackupRepositories
            WHERE @IncludeArchived OR ArchivedAt IS NULL
            ORDER BY Name ASC
            """;

        var result = await db.QueryAsync<BackupRepositoryDto>(
            sql,
            new { IncludeArchived = includeArchived },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupRepositories
                WHERE NormalizedName = @NormalizedName
                  AND ArchivedAt IS NULL)
            """;

        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName }, transaction: tx());
    }

    public Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupRepositories
                WHERE NormalizedName = @NormalizedName
                  AND Id <> @Id
                  AND ArchivedAt IS NULL)
            """;

        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName, Id = id }, transaction: tx());
    }

    public Task<bool> HasActiveOperationAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupRuns
                WHERE BackupRepositoryId = @Id
                  AND Status = ANY(@BackupStatuses)
            )
            OR EXISTS (
                SELECT 1
                FROM BackupRestoreRuns
                WHERE BackupRepositoryId = @Id
                  AND Status = ANY(@RestoreStatuses)
            )
            """;

        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                Id = id,
                BackupStatuses = ActiveBackupStatuses(),
                RestoreStatuses = ActiveRestoreStatuses()
            },
            transaction: tx());
    }

    public Task<bool> HasNonArchivedPolicyAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupPolicies
                WHERE BackupRepositoryId = @Id
                  AND ArchivedAt IS NULL)
            """;

        return db.ExecuteScalarAsync<bool>(sql, new { Id = id }, transaction: tx());
    }

    private static string[] ActiveBackupStatuses() =>
    [
        EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
        EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing),
        EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Running),
        EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.ApplyingRetention)
    ];

    private static string[] ActiveRestoreStatuses() =>
    [
        EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Queued),
        EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Preparing),
        EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Running)
    ];
}

internal sealed class BackupRepositoryValidationRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRepositoryValidationRepository
{
    public Task<int> UpsertAsync(BackupRepositoryValidation validation, CancellationToken cancellationToken)
    {
        validation.Validate();

        const string sql = """
            WITH updated AS (
                UPDATE BackupRepositoryValidations
                SET Status = @Status,
                    LastValidatedAt = @LastValidatedAt,
                    LastErrorCode = @LastErrorCode,
                    LastErrorMessage = @LastErrorMessage
                WHERE BackupRepositoryId = @BackupRepositoryId
                  AND Location = @Location
                  AND ((PlatformId IS NULL AND @PlatformId IS NULL) OR PlatformId = @PlatformId)
                RETURNING 1
            )
            INSERT INTO BackupRepositoryValidations (
                Id, BackupRepositoryId, Location, PlatformId, Status, LastValidatedAt, LastErrorCode, LastErrorMessage)
            SELECT @Id, @BackupRepositoryId, @Location, @PlatformId, @Status, @LastValidatedAt, @LastErrorCode, @LastErrorMessage
            WHERE NOT EXISTS (SELECT 1 FROM updated)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                validation.Id,
                validation.BackupRepositoryId,
                Location = EnumFormatter<BackupExecutionLocation>.GetValue(validation.Location),
                validation.PlatformId,
                Status = EnumFormatter<BackupRepositoryValidationStatus>.GetValue(validation.Status),
                LastValidatedAt = BackupMappers.ToUtcDateTime(validation.LastValidatedAt),
                validation.LastErrorCode,
                validation.LastErrorMessage
            },
            transaction: tx());
    }

    public async Task<BackupRepositoryValidation?> GetAsync(Guid repositoryId, BackupExecutionLocation location, Guid? platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRepositoryValidations
            WHERE BackupRepositoryId = @BackupRepositoryId
              AND Location = @Location
              AND ((PlatformId IS NULL AND @PlatformId IS NULL) OR PlatformId = @PlatformId)
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRepositoryValidationDto>(
            sql,
            new
            {
                BackupRepositoryId = repositoryId,
                Location = EnumFormatter<BackupExecutionLocation>.GetValue(location),
                PlatformId = platformId
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BackupRepositoryValidation>> GetByRepositoryAsync(Guid repositoryId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRepositoryValidations
            WHERE BackupRepositoryId = @BackupRepositoryId
            ORDER BY LastValidatedAt DESC
            """;

        var result = await db.QueryAsync<BackupRepositoryValidationDto>(
            sql,
            new { BackupRepositoryId = repositoryId },
            transaction: tx());
        return result.ToDomain();
    }
}

internal sealed class BackupRepositoryLeaseRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRepositoryLeaseRepository
{
    public async Task<bool> TryAcquireAsync(
        Guid backupRepositoryId,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH deleted_expired AS (
                DELETE FROM BackupRepositoryLeases
                WHERE BackupRepositoryId = @BackupRepositoryId
                  AND ExpiresAt <= @CreatedAt
            ),
            inserted AS (
                INSERT INTO BackupRepositoryLeases (
                    BackupRepositoryId, OperationType, OwnerRunId, ExpiresAt, CreatedAt)
                VALUES (
                    @BackupRepositoryId, @OperationType, @OwnerRunId, @ExpiresAt, @CreatedAt)
                ON CONFLICT (BackupRepositoryId) DO NOTHING
                RETURNING 1
            )
            SELECT EXISTS (SELECT 1 FROM inserted)
            """;

        return await db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                BackupRepositoryId = backupRepositoryId,
                OperationType = operationType,
                OwnerRunId = ownerRunId,
                ExpiresAt = BackupMappers.ToUtcDateTime(expiresAt),
                CreatedAt = BackupMappers.ToUtcDateTime(createdAt)
            },
            transaction: tx());
    }

    public async Task<BackupRepositoryLeaseAcquireResult> TryAcquireForExistingRepositoryAsync(
        Guid backupRepositoryId,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH target AS (
                SELECT Id
                FROM BackupRepositories
                WHERE Id = @BackupRepositoryId
                  AND ArchivedAt IS NULL
            ),
            deleted_expired AS (
                DELETE FROM BackupRepositoryLeases
                WHERE BackupRepositoryId = @BackupRepositoryId
                  AND ExpiresAt <= @CreatedAt
                  AND EXISTS (SELECT 1 FROM target)
            ),
            inserted AS (
                INSERT INTO BackupRepositoryLeases (
                    BackupRepositoryId, OperationType, OwnerRunId, ExpiresAt, CreatedAt)
                SELECT @BackupRepositoryId, @OperationType, @OwnerRunId, @ExpiresAt, @CreatedAt
                WHERE EXISTS (SELECT 1 FROM target)
                ON CONFLICT (BackupRepositoryId) DO NOTHING
                RETURNING 1
            )
            SELECT CASE
                WHEN NOT EXISTS (SELECT 1 FROM target) THEN 1
                WHEN EXISTS (SELECT 1 FROM inserted) THEN 0
                ELSE 2
            END
            """;

        var result = await db.ExecuteScalarAsync<int>(
            sql,
            new
            {
                BackupRepositoryId = backupRepositoryId,
                OperationType = operationType,
                OwnerRunId = ownerRunId,
                ExpiresAt = BackupMappers.ToUtcDateTime(expiresAt),
                CreatedAt = BackupMappers.ToUtcDateTime(createdAt)
            },
            transaction: tx());

        return result switch
        {
            0 => BackupRepositoryLeaseAcquireResult.Acquired,
            1 => BackupRepositoryLeaseAcquireResult.NotFound,
            _ => BackupRepositoryLeaseAcquireResult.Busy
        };
    }

    public Task<int> ReleaseAsync(Guid backupRepositoryId, Guid ownerRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM BackupRepositoryLeases
            WHERE BackupRepositoryId = @BackupRepositoryId
              AND OwnerRunId = @OwnerRunId
            """;

        return db.ExecuteAsync(sql, new { BackupRepositoryId = backupRepositoryId, OwnerRunId = ownerRunId }, transaction: tx());
    }

    public Task<int> DeleteExpiredAsync(DateTimeOffset utcNow, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM BackupRepositoryLeases WHERE ExpiresAt <= @UtcNow";
        return db.ExecuteAsync(sql, new { UtcNow = BackupMappers.ToUtcDateTime(utcNow) }, transaction: tx());
    }
}

internal sealed class BackupSourceLeaseRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupSourceLeaseRepository
{
    public async Task<bool> TryAcquireAsync(
        string sourceKey,
        string operationType,
        Guid ownerRunId,
        DateTimeOffset expiresAt,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH deleted_expired AS (
                DELETE FROM BackupSourceLeases
                WHERE SourceKey = @SourceKey
                  AND ExpiresAt <= @CreatedAt
            ),
            inserted AS (
                INSERT INTO BackupSourceLeases (
                    SourceKey, OperationType, OwnerRunId, ExpiresAt, CreatedAt)
                VALUES (
                    @SourceKey, @OperationType, @OwnerRunId, @ExpiresAt, @CreatedAt)
                ON CONFLICT (SourceKey) DO NOTHING
                RETURNING 1
            )
            SELECT EXISTS (SELECT 1 FROM inserted)
            """;

        return await db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                SourceKey = sourceKey,
                OperationType = operationType,
                OwnerRunId = ownerRunId,
                ExpiresAt = BackupMappers.ToUtcDateTime(expiresAt),
                CreatedAt = BackupMappers.ToUtcDateTime(createdAt)
            },
            transaction: tx());
    }

    public Task<int> ReleaseAsync(string sourceKey, Guid ownerRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM BackupSourceLeases
            WHERE SourceKey = @SourceKey
              AND OwnerRunId = @OwnerRunId
            """;

        return db.ExecuteAsync(sql, new { SourceKey = sourceKey, OwnerRunId = ownerRunId }, transaction: tx());
    }

    public Task<int> DeleteExpiredAsync(DateTimeOffset utcNow, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM BackupSourceLeases WHERE ExpiresAt <= @UtcNow";
        return db.ExecuteAsync(sql, new { UtcNow = BackupMappers.ToUtcDateTime(utcNow) }, transaction: tx());
    }
}

internal sealed class BackupPolicyRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupPolicyRepository
{
    public async Task<int> AddAsync(
        BackupPolicy policy,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? tagCreatedByActorId = null)
    {
        policy.Validate();

        string sql = ResourceTagSql.InputTagsCte + """
            inserted_policy AS (
                INSERT INTO BackupPolicies (
                    Id, Name, NormalizedName, Description, Source, BackupRepositoryId, Enabled,
                    Cron, TimeZone, KeepLastSuccessful, TimeoutSeconds, AlertOnFailure, RunAsActorId,
                    ControlState, CurrentRunId, LastScheduledRunAt, FirstSuccessfulRunAt,
                    CreatedByActorId, CreatedAt, UpdatedAt, ArchivedAt, RowVersion)
                SELECT
                    @Id, @Name, @NormalizedName, @Description, @Source::jsonb, @BackupRepositoryId, @Enabled,
                    @Cron, @TimeZone, @KeepLastSuccessful, @TimeoutSeconds, @AlertOnFailure, @RunAsActorId,
                    @ControlState, @CurrentRunId, @LastScheduledRunAt, @FirstSuccessfulRunAt,
                    @CreatedByActorId, @CreatedAt, @UpdatedAt, @ArchivedAt, @RowVersion
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_policy", "p") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_policy", "inserted_policy", "inserted_tags");

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(
            sql,
            new
            {
                policy.Id,
                policy.Name,
                policy.NormalizedName,
                policy.Description,
                Source = BackupMappers.SerializeSource(policy.Source),
                policy.BackupRepositoryId,
                policy.Enabled,
                policy.Cron,
                policy.TimeZone,
                policy.KeepLastSuccessful,
                policy.TimeoutSeconds,
                policy.AlertOnFailure,
                policy.RunAsActorId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(policy.ControlState),
                policy.CurrentRunId,
                LastScheduledRunAt = BackupMappers.ToUtcDateTime(policy.LastScheduledRunAt),
                FirstSuccessfulRunAt = BackupMappers.ToUtcDateTime(policy.FirstSuccessfulRunAt),
                policy.CreatedByActorId,
                CreatedAt = BackupMappers.ToUtcDateTime(policy.CreatedAt),
                UpdatedAt = BackupMappers.ToUtcDateTime(policy.UpdatedAt),
                ArchivedAt = BackupMappers.ToUtcDateTime(policy.ArchivedAt),
                policy.RowVersion,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy),
                TagIds = tagIdArray,
                TagCreatedByActorId = tagCreatedByActorId ?? policy.CreatedByActorId
            },
            transaction: tx());

        policy.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public Task<int> UpdateAsync(BackupPolicy policy, CancellationToken cancellationToken)
    {
        policy.Validate();

        const string sql = """
            UPDATE BackupPolicies
            SET Name = @Name,
                NormalizedName = @NormalizedName,
                Description = @Description,
                Source = @Source::jsonb,
                BackupRepositoryId = @BackupRepositoryId,
                Enabled = @Enabled,
                Cron = @Cron,
                TimeZone = @TimeZone,
                KeepLastSuccessful = @KeepLastSuccessful,
                TimeoutSeconds = @TimeoutSeconds,
                AlertOnFailure = @AlertOnFailure,
                RunAsActorId = @RunAsActorId,
                ControlState = @ControlState,
                CurrentRunId = @CurrentRunId,
                LastScheduledRunAt = @LastScheduledRunAt,
                FirstSuccessfulRunAt = @FirstSuccessfulRunAt,
                UpdatedAt = @UpdatedAt,
                ArchivedAt = @ArchivedAt,
                RowVersion = @RowVersion
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                policy.Id,
                policy.Name,
                policy.NormalizedName,
                policy.Description,
                Source = BackupMappers.SerializeSource(policy.Source),
                policy.BackupRepositoryId,
                policy.Enabled,
                policy.Cron,
                policy.TimeZone,
                policy.KeepLastSuccessful,
                policy.TimeoutSeconds,
                policy.AlertOnFailure,
                policy.RunAsActorId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(policy.ControlState),
                policy.CurrentRunId,
                LastScheduledRunAt = BackupMappers.ToUtcDateTime(policy.LastScheduledRunAt),
                FirstSuccessfulRunAt = BackupMappers.ToUtcDateTime(policy.FirstSuccessfulRunAt),
                UpdatedAt = BackupMappers.ToUtcDateTime(policy.UpdatedAt),
                ArchivedAt = BackupMappers.ToUtcDateTime(policy.ArchivedAt),
                policy.RowVersion
            },
            transaction: tx());
    }

    public Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET Enabled = FALSE,
                ArchivedAt = COALESCE(ArchivedAt, @ArchivedAt),
                UpdatedAt = @ArchivedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(sql, new { Id = id, ArchivedAt = BackupMappers.ToUtcDateTime(archivedAt) }, transaction: tx());
    }

    public async Task<BackupPolicy?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BackupPolicies p
            WHERE p.Id = @Id
              AND (@IncludeArchived OR p.ArchivedAt IS NULL)
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupPolicyDto>(
            sql,
            new
            {
                Id = id,
                IncludeArchived = includeArchived,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy)
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BackupPolicy>> GetAllAsync(
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        bool includeArchived = false)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BackupPolicies p
            WHERE (@IncludeArchived OR p.ArchivedAt IS NULL)
              AND {{ResourceTagSql.FilterPredicate("p")}}
            ORDER BY p.Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<BackupPolicyDto>(
            sql,
            new
            {
                IncludeArchived = includeArchived,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<BackupPolicy>> GetAuthorizedAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BackupPolicies p
            WHERE p.ArchivedAt IS NULL
              AND {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}}
              AND {{ResourceTagSql.FilterPredicate("p")}}
            ORDER BY p.Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<BackupPolicyDto>(
            sql,
            new
            {
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupPolicies
                WHERE NormalizedName = @NormalizedName
                  AND ArchivedAt IS NULL)
            """;

        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName }, transaction: tx());
    }

    public Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupPolicies
                WHERE NormalizedName = @NormalizedName
                  AND Id <> @Id
                  AND ArchivedAt IS NULL)
            """;

        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName, Id = id }, transaction: tx());
    }

    public Task<bool> CanAccessAsync(
        Guid userId,
        Guid id,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT EXISTS (
                SELECT 1
                FROM BackupPolicies p
                WHERE p.Id = @Id
                  AND p.ArchivedAt IS NULL
                  AND {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}}
            )
            """;

        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                Id = id,
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission
            },
            transaction: tx());
    }

    public Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET ControlState = @ControlState,
                CurrentRunId = @RunId,
                UpdatedAt = @UpdatedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                RunId = runId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                UpdatedAt = DateTime.UtcNow
            },
            transaction: tx());
    }

    public Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET ControlState = @ControlState,
                CurrentRunId = NULL,
                UpdatedAt = @UpdatedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND CurrentRunId = @RunId
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                RunId = runId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle),
                UpdatedAt = DateTime.UtcNow
            },
            transaction: tx());
    }

    public Task<int> MarkIdleAfterRunAsync(Guid id, Guid runId, bool successful, DateTimeOffset completedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET ControlState = @ControlState,
                CurrentRunId = NULL,
                FirstSuccessfulRunAt = CASE
                    WHEN @Successful THEN COALESCE(FirstSuccessfulRunAt, @CompletedAt)
                    ELSE FirstSuccessfulRunAt
                END,
                UpdatedAt = @CompletedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND CurrentRunId = @RunId
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                RunId = runId,
                Successful = successful,
                CompletedAt = BackupMappers.ToUtcDateTime(completedAt),
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle)
            },
            transaction: tx());
    }

}

internal sealed class BackupRunRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRunRepository
{
    private const string ExecutionPlanSelect = """
            r.Id AS RunId,
            r.BackupPolicyId AS RunBackupPolicyId,
            r.BackupRepositoryId AS RunBackupRepositoryId,
            r.PolicyNameSnapshot AS RunPolicyNameSnapshot,
            r.SourceSnapshot AS RunSourceSnapshot,
            r.RepositoryTypeSnapshot AS RunRepositoryTypeSnapshot,
            r.Trigger AS RunTrigger,
            r.TriggerSourceId AS RunTriggerSourceId,
            r.Status AS RunStatus,
            r.ResticSnapshotId AS RunResticSnapshotId,
            r.ParentSnapshotId AS RunParentSnapshotId,
            r.SnapshotAvailability AS RunSnapshotAvailability,
            r.FilesProcessed AS RunFilesProcessed,
            r.BytesProcessed AS RunBytesProcessed,
            r.BytesAdded AS RunBytesAdded,
            r.Warnings AS RunWarnings,
            r.QueuedAt AS RunQueuedAt,
            r.StartedAt AS RunStartedAt,
            r.CompletedAt AS RunCompletedAt,
            r.ExitCode AS RunExitCode,
            r.ErrorCode AS RunErrorCode,
            r.ErrorMessage AS RunErrorMessage,
            r.TriggeredByActorId AS RunTriggeredByActorId,
            p.Id AS PolicyId,
            p.Name AS PolicyName,
            p.NormalizedName AS PolicyNormalizedName,
            p.Description AS PolicyDescription,
            p.Source AS PolicySource,
            p.BackupRepositoryId AS PolicyBackupRepositoryId,
            p.Enabled AS PolicyEnabled,
            p.Cron AS PolicyCron,
            p.TimeZone AS PolicyTimeZone,
            p.KeepLastSuccessful AS PolicyKeepLastSuccessful,
            p.TimeoutSeconds AS PolicyTimeoutSeconds,
            p.AlertOnFailure AS PolicyAlertOnFailure,
            p.RunAsActorId AS PolicyRunAsActorId,
            p.ControlState AS PolicyControlState,
            p.CurrentRunId AS PolicyCurrentRunId,
            p.LastScheduledRunAt AS PolicyLastScheduledRunAt,
            p.FirstSuccessfulRunAt AS PolicyFirstSuccessfulRunAt,
            p.CreatedByActorId AS PolicyCreatedByActorId,
            p.CreatedAt AS PolicyCreatedAt,
            p.UpdatedAt AS PolicyUpdatedAt,
            p.ArchivedAt AS PolicyArchivedAt,
            p.RowVersion AS PolicyRowVersion,
            br.Id AS RepositoryId,
            br.Name AS RepositoryName,
            br.NormalizedName AS RepositoryNormalizedName,
            br.Description AS RepositoryDescription,
            br.Type AS RepositoryType,
            br.Spec AS RepositorySpec,
            br.PasswordSecretId AS RepositoryPasswordSecretId,
            br.Status AS RepositoryStatus,
            br.LastPrunedAt AS RepositoryLastPrunedAt,
            br.LastCheckedAt AS RepositoryLastCheckedAt,
            br.CreatedByActorId AS RepositoryCreatedByActorId,
            br.CreatedAt AS RepositoryCreatedAt,
            br.UpdatedAt AS RepositoryUpdatedAt,
            br.ArchivedAt AS RepositoryArchivedAt,
            br.RowVersion AS RepositoryRowVersion
        """;

    public Task<int> AddAsync(BackupRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO BackupRuns (
                Id, BackupPolicyId, BackupRepositoryId, PolicyNameSnapshot, SourceSnapshot, RepositoryTypeSnapshot,
                Trigger, TriggerSourceId, Status, ResticSnapshotId, ParentSnapshotId, SnapshotAvailability,
                FilesProcessed, BytesProcessed, BytesAdded, Warnings, QueuedAt, StartedAt, CompletedAt,
                ExitCode, ErrorCode, ErrorMessage, TriggeredByActorId)
            VALUES (
                @Id, @BackupPolicyId, @BackupRepositoryId, @PolicyNameSnapshot, @SourceSnapshot::jsonb, @RepositoryTypeSnapshot,
                @Trigger, @TriggerSourceId, @Status, @ResticSnapshotId, @ParentSnapshotId, @SnapshotAvailability,
                @FilesProcessed, @BytesProcessed, @BytesAdded, @Warnings::jsonb, @QueuedAt, @StartedAt, @CompletedAt,
                @ExitCode, @ErrorCode, @ErrorMessage, @TriggeredByActorId)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                run.BackupPolicyId,
                run.BackupRepositoryId,
                run.PolicyNameSnapshot,
                SourceSnapshot = BackupMappers.SerializeSource(run.SourceSnapshot),
                RepositoryTypeSnapshot = EnumFormatter<BackupRepositoryType>.GetValue(run.RepositoryTypeSnapshot),
                Trigger = EnumFormatter<BackupRunTrigger>.GetValue(run.Trigger),
                run.TriggerSourceId,
                Status = EnumFormatter<BackupRunStatus>.GetValue(run.Status),
                run.ResticSnapshotId,
                run.ParentSnapshotId,
                SnapshotAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(run.SnapshotAvailability),
                run.FilesProcessed,
                run.BytesProcessed,
                run.BytesAdded,
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                QueuedAt = BackupMappers.ToUtcDateTime(run.QueuedAt),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(run.CompletedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage,
                run.TriggeredByActorId
            },
            transaction: tx());
    }

    public async Task<BackupRunQueueResult> QueueAsync(
        Guid policyId,
        Guid runId,
        BackupRunTrigger trigger,
        Guid? triggerSourceId,
        Guid triggeredByActorId,
        bool usePolicyActor,
        DateTimeOffset queuedAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH policy AS (
                SELECT
                    p.Id AS BackupPolicyId,
                    p.BackupRepositoryId,
                    p.Name AS PolicyNameSnapshot,
                    p.Source AS SourceSnapshot,
                    p.RunAsActorId,
                    p.ArchivedAt,
                    r.Type AS RepositoryTypeSnapshot
                FROM BackupPolicies p
                JOIN BackupRepositories r ON r.Id = p.BackupRepositoryId
                WHERE p.Id = @PolicyId
                LIMIT 1
            ),
            inserted AS (
                INSERT INTO BackupRuns (
                    Id, BackupPolicyId, BackupRepositoryId, PolicyNameSnapshot, SourceSnapshot, RepositoryTypeSnapshot,
                    Trigger, TriggerSourceId, Status, ResticSnapshotId, ParentSnapshotId, SnapshotAvailability,
                    FilesProcessed, BytesProcessed, BytesAdded, Warnings, QueuedAt, StartedAt, CompletedAt,
                    ExitCode, ErrorCode, ErrorMessage, TriggeredByActorId)
                SELECT
                    @RunId, p.BackupPolicyId, p.BackupRepositoryId, p.PolicyNameSnapshot, p.SourceSnapshot, p.RepositoryTypeSnapshot,
                    @Trigger, @TriggerSourceId, @Status, NULL, NULL, @SnapshotAvailability,
                    NULL, NULL, NULL, @Warnings::jsonb, @QueuedAt, NULL, NULL,
                    NULL, NULL, NULL,
                    CASE WHEN @UsePolicyActor THEN p.RunAsActorId ELSE @TriggeredByActorId END
                FROM policy p
                WHERE p.ArchivedAt IS NULL
                ON CONFLICT (BackupPolicyId)
                    WHERE Status IN ('Queued', 'Preparing', 'Running', 'ApplyingRetention')
                    DO NOTHING
                RETURNING *
            )
            SELECT
                CASE
                    WHEN EXISTS (SELECT 1 FROM inserted) THEN 0
                    WHEN NOT EXISTS (SELECT 1 FROM policy) THEN 1
                    WHEN EXISTS (SELECT 1 FROM policy WHERE ArchivedAt IS NOT NULL) THEN 2
                    ELSE 3
                END AS ResultStatus,
                i.*
            FROM inserted i
            RIGHT JOIN (SELECT 1) s ON TRUE
            """;

        var result = await db.QuerySingleAsync<BackupRunQueueDto>(
            sql,
            new
            {
                PolicyId = policyId,
                RunId = runId,
                Trigger = EnumFormatter<BackupRunTrigger>.GetValue(trigger),
                TriggerSourceId = triggerSourceId,
                Status = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                SnapshotAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(BackupSnapshotAvailability.Pending),
                Warnings = BackupMappers.SerializeWarnings([]),
                QueuedAt = BackupMappers.ToUtcDateTime(queuedAt),
                TriggeredByActorId = triggeredByActorId,
                UsePolicyActor = usePolicyActor
            },
            transaction: tx());

        return result.ResultStatus switch
        {
            0 => new BackupRunQueueResult(BackupRunQueueResultStatus.Queued, result.ToDomain()),
            1 => new BackupRunQueueResult(BackupRunQueueResultStatus.PolicyNotFound, null),
            2 => new BackupRunQueueResult(BackupRunQueueResultStatus.PolicyArchived, null),
            _ => new BackupRunQueueResult(BackupRunQueueResultStatus.ActiveRunExists, null)
        };
    }

    public Task<int> UpdateAsync(BackupRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRuns
            SET Status = @Status,
                ResticSnapshotId = @ResticSnapshotId,
                ParentSnapshotId = @ParentSnapshotId,
                SnapshotAvailability = @SnapshotAvailability,
                FilesProcessed = @FilesProcessed,
                BytesProcessed = @BytesProcessed,
                BytesAdded = @BytesAdded,
                Warnings = @Warnings::jsonb,
                StartedAt = @StartedAt,
                CompletedAt = @CompletedAt,
                ExitCode = @ExitCode,
                ErrorCode = @ErrorCode,
                ErrorMessage = @ErrorMessage
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                Status = EnumFormatter<BackupRunStatus>.GetValue(run.Status),
                run.ResticSnapshotId,
                run.ParentSnapshotId,
                SnapshotAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(run.SnapshotAvailability),
                run.FilesProcessed,
                run.BytesProcessed,
                run.BytesAdded,
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(run.CompletedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage
            },
            transaction: tx());
    }

    public async Task<BackupRunFinishResult> FinishRunAndMarkPolicyIdleAsync(
        BackupRun run,
        Guid policyId,
        bool successful,
        DateTimeOffset completedAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH updated_run AS (
                UPDATE BackupRuns
                SET Status = @Status,
                    ResticSnapshotId = @ResticSnapshotId,
                    ParentSnapshotId = @ParentSnapshotId,
                    SnapshotAvailability = @SnapshotAvailability,
                    FilesProcessed = @FilesProcessed,
                    BytesProcessed = @BytesProcessed,
                    BytesAdded = @BytesAdded,
                    Warnings = @Warnings::jsonb,
                    StartedAt = @StartedAt,
                    CompletedAt = @CompletedAt,
                    ExitCode = @ExitCode,
                    ErrorCode = @ErrorCode,
                    ErrorMessage = @ErrorMessage
                WHERE Id = @Id
                  AND (@Status = @CancelledStatus OR Status <> @CancelledStatus)
                RETURNING 1
            ),
            updated_policy AS (
                UPDATE BackupPolicies
                SET ControlState = @IdleControlState,
                    CurrentRunId = NULL,
                    FirstSuccessfulRunAt = CASE
                        WHEN @Successful AND EXISTS (SELECT 1 FROM updated_run) THEN COALESCE(FirstSuccessfulRunAt, @CompletedAt)
                        ELSE FirstSuccessfulRunAt
                    END,
                    UpdatedAt = @CompletedAt,
                    RowVersion = RowVersion + 1
                WHERE Id = @PolicyId
                  AND CurrentRunId = @Id
                RETURNING 1
            )
            SELECT CASE
                WHEN EXISTS (SELECT 1 FROM updated_run) THEN 0
                ELSE 1
            END
            """;

        var result = await db.ExecuteScalarAsync<int>(
            sql,
            new
            {
                run.Id,
                PolicyId = policyId,
                Successful = successful,
                Status = EnumFormatter<BackupRunStatus>.GetValue(run.Status),
                CancelledStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Cancelled),
                run.ResticSnapshotId,
                run.ParentSnapshotId,
                SnapshotAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(run.SnapshotAvailability),
                run.FilesProcessed,
                run.BytesProcessed,
                run.BytesAdded,
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(completedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage,
                IdleControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle)
            },
            transaction: tx());

        return result == 0 ? BackupRunFinishResult.Completed : BackupRunFinishResult.AlreadyCancelled;
    }

    public async Task<BackupRun?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM BackupRuns WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<BackupRunDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<BackupRunExecutionPlan?> GetExecutionPlanAsync(Guid id, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT
                {{ExecutionPlanSelect}}
            FROM BackupRuns r
            JOIN BackupPolicies p ON p.Id = r.BackupPolicyId
            JOIN BackupRepositories br ON br.Id = r.BackupRepositoryId
            WHERE r.Id = @Id
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRunExecutionPlanDto>(
            sql,
            new { Id = id },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<BackupRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken)
    {
        string sql = $$"""
            WITH claimed AS (
                UPDATE BackupRuns
                SET Status = @PreparingStatus,
                    StartedAt = COALESCE(StartedAt, @StartedAt)
                WHERE Id = @Id
                  AND Status = @QueuedStatus
                RETURNING *
            ),
            updated_policy AS (
                UPDATE BackupPolicies p
                SET ControlState = @ProcessingControlState,
                    CurrentRunId = @Id,
                    UpdatedAt = @StartedAt,
                    RowVersion = RowVersion + 1
                FROM claimed c
                WHERE p.Id = c.BackupPolicyId
                RETURNING p.*
            )
            SELECT
                {{ExecutionPlanSelect}}
            FROM claimed r
            JOIN updated_policy p ON p.Id = r.BackupPolicyId
            JOIN BackupRepositories br ON br.Id = r.BackupRepositoryId
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRunExecutionPlanDto>(
            sql,
            new
            {
                Id = id,
                StartedAt = BackupMappers.ToUtcDateTime(startedAt),
                QueuedStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                PreparingStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing),
                ProcessingControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing)
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BackupRun>> GetByPolicyAsync(Guid policyId, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRuns
            WHERE BackupPolicyId = @PolicyId
            ORDER BY QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRunDto>(
            sql,
            new { PolicyId = policyId, Limit = Math.Clamp(limit, 1, 200) },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<IReadOnlyList<Guid>> GetQueuedIdsAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id
            FROM BackupRuns
            WHERE Status = @Status
            ORDER BY QueuedAt ASC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<Guid>(
            sql,
            new
            {
                Status = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                Limit = Math.Clamp(limit, 1, 100)
            },
            transaction: tx());
        return [.. result];
    }

    public async Task<IEnumerable<BackupRun>> GetPagedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRuns
            ORDER BY QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRunDto>(
            sql,
            new { Limit = Math.Clamp(limit, 1, 500) },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<BackupRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRuns
            WHERE Status = @Status
            ORDER BY QueuedAt ASC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRunDto>(
            sql,
            new
            {
                Status = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                Limit = Math.Clamp(limit, 1, 100)
            },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<bool> HasActiveRunAsync(Guid policyId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM BackupRuns
                WHERE BackupPolicyId = @PolicyId
                  AND Status = ANY(@Statuses)
            )
            """;

        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                PolicyId = policyId,
                Statuses = new[]
                {
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Running),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.ApplyingRetention)
                }
            },
            transaction: tx());
    }

    public async Task<bool> TryMarkPreparingAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRuns
            SET Status = @PreparingStatus,
                StartedAt = COALESCE(StartedAt, @StartedAt)
            WHERE Id = @Id
              AND Status = @QueuedStatus
            """;

        var rows = await db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                StartedAt = BackupMappers.ToUtcDateTime(startedAt),
                QueuedStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                PreparingStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing)
            },
            transaction: tx());
        return rows > 0;
    }

    public Task<int> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRuns
            SET Status = @CancelledStatus,
                SnapshotAvailability = CASE
                    WHEN SnapshotAvailability = @PendingAvailability THEN @NotCreatedAvailability
                    ELSE SnapshotAvailability
                END,
                CompletedAt = @CancelledAt,
                ErrorCode = @ErrorCode,
                ErrorMessage = @Reason
            WHERE Id = @Id
              AND Status = ANY(@ActiveStatuses)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                CancelledAt = BackupMappers.ToUtcDateTime(cancelledAt),
                ErrorCode = "backup.cancelled",
                Reason = reason,
                CancelledStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Cancelled),
                PendingAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(BackupSnapshotAvailability.Pending),
                NotCreatedAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(BackupSnapshotAvailability.NotCreated),
                ActiveStatuses = new[]
                {
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Running),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.ApplyingRetention)
                }
            },
            transaction: tx());
    }

}

internal sealed class BackupRunLogRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRunLogRepository
{
    public Task<int> AddAsync(BackupRunLogEntry entry, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO BackupRunLogs (Id, BackupRunId, CreatedAt, Stream, Message)
            VALUES (@Id, @BackupRunId, @CreatedAt, @Stream, @Message)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                entry.Id,
                entry.BackupRunId,
                CreatedAt = BackupMappers.ToUtcDateTime(entry.CreatedAt),
                entry.Stream,
                entry.Message
            },
            transaction: tx());
    }

    public Task<int> AddRangeAsync(IReadOnlyCollection<BackupRunLogEntry> entries, CancellationToken cancellationToken)
    {
        if (entries.Count == 0)
            return Task.FromResult(0);

        var batch = entries.ToArray();
        const string sql = """
            INSERT INTO BackupRunLogs (Id, BackupRunId, CreatedAt, Stream, Message)
            SELECT Id, BackupRunId, CreatedAt, Stream, Message
            FROM unnest(
                @Ids::uuid[],
                @BackupRunIds::uuid[],
                @CreatedAts::timestamp[],
                @Streams::text[],
                @Messages::text[])
                AS logs(Id, BackupRunId, CreatedAt, Stream, Message)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = batch.Select(static entry => entry.Id).ToArray(),
                BackupRunIds = batch.Select(static entry => entry.BackupRunId).ToArray(),
                CreatedAts = batch.Select(static entry => BackupMappers.ToUtcDateTime(entry.CreatedAt)).ToArray(),
                Streams = batch.Select(static entry => entry.Stream).ToArray(),
                Messages = batch.Select(static entry => entry.Message).ToArray()
            },
            transaction: tx());
    }

    public async Task<IReadOnlyList<BackupRunLogEntry>> GetByRunAsync(Guid backupRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRunLogs
            WHERE BackupRunId = @BackupRunId
            ORDER BY CreatedAt ASC, Id ASC
            """;

        var result = await db.QueryAsync<BackupRunLogDto>(sql, new { BackupRunId = backupRunId }, transaction: tx());
        return result.ToDomain();
    }
}

internal sealed class BackupRestoreRunRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRestoreRunRepository
{
    public Task<int> AddAsync(BackupRestoreRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO BackupRestoreRuns (
                Id, BackupRunId, BackupRepositoryId, Status, TargetPlatformId, TargetVolumeName,
                OverwriteExisting, TargetVolumeCreatedByCitadel, AffectedContainers, Warnings,
                QueuedAt, StartedAt, CompletedAt, ExitCode, ErrorCode, ErrorMessage, TriggeredByActorId)
            VALUES (
                @Id, @BackupRunId, @BackupRepositoryId, @Status, @TargetPlatformId, @TargetVolumeName,
                @OverwriteExisting, @TargetVolumeCreatedByCitadel, @AffectedContainers::jsonb, @Warnings::jsonb,
                @QueuedAt, @StartedAt, @CompletedAt, @ExitCode, @ErrorCode, @ErrorMessage, @TriggeredByActorId)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                run.BackupRunId,
                run.BackupRepositoryId,
                Status = EnumFormatter<BackupRestoreStatus>.GetValue(run.Status),
                run.TargetPlatformId,
                run.TargetVolumeName,
                run.OverwriteExisting,
                run.TargetVolumeCreatedByCitadel,
                AffectedContainers = BackupMappers.SerializeAffectedContainers(run.AffectedContainers),
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                QueuedAt = BackupMappers.ToUtcDateTime(run.QueuedAt),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(run.CompletedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage,
                run.TriggeredByActorId
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(BackupRestoreRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRestoreRuns
            SET Status = @Status,
                TargetVolumeCreatedByCitadel = @TargetVolumeCreatedByCitadel,
                AffectedContainers = @AffectedContainers::jsonb,
                Warnings = @Warnings::jsonb,
                StartedAt = @StartedAt,
                CompletedAt = @CompletedAt,
                ExitCode = @ExitCode,
                ErrorCode = @ErrorCode,
                ErrorMessage = @ErrorMessage
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                Status = EnumFormatter<BackupRestoreStatus>.GetValue(run.Status),
                run.TargetVolumeCreatedByCitadel,
                AffectedContainers = BackupMappers.SerializeAffectedContainers(run.AffectedContainers),
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(run.CompletedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage
            },
            transaction: tx());
    }

    public async Task<BackupRestoreRun?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM BackupRestoreRuns WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<BackupRestoreRunDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BackupRestoreRun>> GetPagedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRestoreRuns
            ORDER BY QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRestoreRunDto>(
            sql,
            new { Limit = Math.Clamp(limit, 1, 500) },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<BackupRestoreRun>> GetByBackupRunAsync(Guid backupRunId, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRestoreRuns
            WHERE BackupRunId = @BackupRunId
            ORDER BY QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRestoreRunDto>(
            sql,
            new { BackupRunId = backupRunId, Limit = Math.Clamp(limit, 1, 200) },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<int> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRestoreRuns
            SET Status = @CancelledStatus,
                CompletedAt = @CancelledAt,
                ErrorCode = @ErrorCode,
                ErrorMessage = @Reason
            WHERE Id = @Id
              AND Status = ANY(@ActiveStatuses)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                CancelledAt = BackupMappers.ToUtcDateTime(cancelledAt),
                ErrorCode = "backup.restore.cancelled",
                Reason = reason,
                CancelledStatus = EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Cancelled),
                ActiveStatuses = new[]
                {
                    EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Queued),
                    EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Preparing),
                    EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Running)
                }
            },
            transaction: tx());
    }

}
