using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Backups;
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
                ControlState, CurrentRunId, ControlStartedAt, LastPrunedAt, LastCheckedAt,
                CreatedByActorId, CreatedAt, UpdatedAt, ArchivedAt, RowVersion)
            VALUES (
                @Id, @Name, @NormalizedName, @Description, @Type, @Spec::jsonb, @PasswordSecretId, @Status,
                @ControlState, @CurrentRunId, @ControlStartedAt, @LastPrunedAt, @LastCheckedAt,
                @CreatedByActorId, @CreatedAt, @UpdatedAt, @ArchivedAt, @RowVersion)
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
                ControlState = EnumFormatter<ResourceControlState>.GetValue(repository.ControlState),
                repository.CurrentRunId,
                repository.ControlStartedAt,
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
                ControlState = @ControlState,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @ControlStartedAt,
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
                ControlState = EnumFormatter<ResourceControlState>.GetValue(repository.ControlState),
                repository.CurrentRunId,
                repository.ControlStartedAt,
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
            active_repository_operation AS (
                SELECT 1
                FROM BackupRepositories
                WHERE Id = @Id
                  AND ControlState = @ProcessingControlState
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
                  AND NOT EXISTS (SELECT 1 FROM active_repository_operation)
                  AND NOT EXISTS (SELECT 1 FROM active_policies)
                RETURNING 1
            )
            SELECT CASE
                WHEN EXISTS (SELECT 1 FROM archived) THEN 0
                WHEN NOT EXISTS (SELECT 1 FROM target) THEN 1
                WHEN EXISTS (SELECT 1 FROM active_operations)
                  OR EXISTS (SELECT 1 FROM active_restores)
                  OR EXISTS (SELECT 1 FROM active_repository_operation) THEN 2
                ELSE 3
            END
            """;

        var result = await db.ExecuteScalarAsync<int>(
            sql,
            new
            {
                Id = id,
                ArchivedAt = BackupMappers.ToUtcDateTime(archivedAt),
                ProcessingControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
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

    public Task<IEnumerable<ResourceInfo>> GetAuthorizedLookupAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT r.Id, r.Name
            FROM BackupRepositories r
            WHERE r.ArchivedAt IS NULL
              AND {{AuthorizationSql.ResourcePredicatePrefix}}r.Id{{AuthorizationSql.ResourcePredicateSuffix}}
            ORDER BY r.Name ASC
            """;

        return db.QueryAsync<ResourceInfo>(
            sql,
            new
            {
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission
            },
            transaction: tx());
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

    public Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRepositories
            SET ControlState = @ControlState,
                CurrentRunId = @RunId,
                ControlStartedAt = @ControlStartedAt,
                UpdatedAt = @UpdatedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND ArchivedAt IS NULL
            """;

        var updatedAt = DateTime.UtcNow;
        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                RunId = runId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                UpdatedAt = updatedAt
            },
            transaction: tx());
    }

    public Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRepositories
            SET ControlState = @ControlState,
                CurrentRunId = NULL,
                ControlStartedAt = NULL,
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

    public async Task<IEnumerable<BackupRepository>> GetStuckRepositoriesAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default)
    {
        const string sql = """
            SELECT br.*
            FROM BackupRepositories br
            LEFT JOIN BackupRuns r ON r.Id = br.CurrentRunId
            WHERE br.ArchivedAt IS NULL
              AND br.ControlState = @ControlState
              AND br.ControlStartedAt IS NOT NULL
              AND br.ControlStartedAt < @ControlStartedAt
              AND (
                  r.Id IS NULL
                  OR r.Status <> ALL(@ActiveStatuses)
              )
            ORDER BY br.ControlStartedAt ASC
            """;

        var result = await db.QueryAsync<BackupRepositoryDto>(
            sql,
            new
            {
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - staleAfterSeconds,
                ActiveStatuses = ActiveBackupStatuses()
            },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateProcessingAsync(
        Guid id,
        ResourceControlState state,
        long? startedAt,
        long rowVersion,
        bool checkRowVersion,
        Guid? currentRunId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRepositories
            SET ControlState = @State,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @StartedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND (@CheckRowVersion = false OR RowVersion = @RowVersion)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                CurrentRunId = currentRunId,
                StartedAt = startedAt,
                RowVersion = rowVersion,
                CheckRowVersion = checkRowVersion
            },
            transaction: tx());
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
                    Cron, TimeZone, Webhook, KeepLastSuccessful, TimeoutSeconds, AlertOnFailure, RunAsActorId,
                    ControlState, CurrentRunId, ControlStartedAt, LastScheduledRunAt, FirstSuccessfulRunAt,
                    CreatedByActorId, CreatedAt, UpdatedAt, ArchivedAt, RowVersion)
                SELECT
                    @Id, @Name, @NormalizedName, @Description, @Source::jsonb, @BackupRepositoryId, @Enabled,
                    @Cron, @TimeZone, @Webhook::jsonb, @KeepLastSuccessful, @TimeoutSeconds, @AlertOnFailure, @RunAsActorId,
                    @ControlState, @CurrentRunId, @ControlStartedAt, @LastScheduledRunAt, @FirstSuccessfulRunAt,
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
                Webhook = BackupMappers.SerializeWebhook(policy.Webhook),
                policy.KeepLastSuccessful,
                policy.TimeoutSeconds,
                policy.AlertOnFailure,
                policy.RunAsActorId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(policy.ControlState),
                policy.CurrentRunId,
                policy.ControlStartedAt,
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
                Webhook = @Webhook::jsonb,
                KeepLastSuccessful = @KeepLastSuccessful,
                TimeoutSeconds = @TimeoutSeconds,
                AlertOnFailure = @AlertOnFailure,
                RunAsActorId = @RunAsActorId,
                ControlState = @ControlState,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @ControlStartedAt,
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
                Webhook = BackupMappers.SerializeWebhook(policy.Webhook),
                policy.KeepLastSuccessful,
                policy.TimeoutSeconds,
                policy.AlertOnFailure,
                policy.RunAsActorId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(policy.ControlState),
                policy.CurrentRunId,
                policy.ControlStartedAt,
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

    public async Task<IEnumerable<ScheduledBackupPolicy>> GetScheduledAsync(DateTimeOffset scheduledMinuteUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT p.Id,
                   p.Cron,
                   p.TimeZone
            FROM BackupPolicies p
            WHERE p.ArchivedAt IS NULL
              AND p.Enabled
              AND p.Cron IS NOT NULL
              AND (p.LastScheduledRunAt IS NULL OR p.LastScheduledRunAt < @ScheduledMinuteUtc)
            """;

        var result = await db.QueryAsync<ScheduledBackupPolicyDto>(
            sql,
            new { ScheduledMinuteUtc = BackupMappers.ToUtcDateTime(scheduledMinuteUtc) },
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

    public async Task<IReadOnlyList<VolumeBackupCoverage>> GetVolumeCoverageAsync(
        IReadOnlyCollection<VolumeBackupCoverageKey> volumes,
        Guid? userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        if (volumes.Count == 0)
            return [];

        var requested = volumes
            .Where(static volume => volume.PlatformId != Guid.Empty && !string.IsNullOrWhiteSpace(volume.VolumeName))
            .Select(static volume => new VolumeBackupCoverageKey(volume.PlatformId, BackupRepository.NormalizeName(volume.VolumeName)))
            .Distinct()
            .ToArray();

        if (requested.Length == 0)
            return [];

        var authorizationCtes = userId.HasValue
            ? $", {AuthorizationSql.ActorScopeCte}, {AuthorizationSql.GlobalAccessCte}"
            : string.Empty;
        var authorizationPredicate = userId.HasValue
            ? $"AND {AuthorizationSql.ResourcePredicatePrefix}p.Id{AuthorizationSql.ResourcePredicateSuffix}"
            : string.Empty;

        string sql = $$"""
            WITH requested AS (
                SELECT *
                FROM unnest(@PlatformIds::uuid[], @VolumeNames::text[]) AS r(PlatformId, VolumeName)
            )
            {{authorizationCtes}},
            policies AS (
                SELECT
                    r.PlatformId,
                    r.VolumeName,
                    p.Id,
                    p.Enabled,
                    p.BackupRepositoryId
                FROM requested r
                JOIN BackupPolicies p
                  ON p.ArchivedAt IS NULL
                 AND (
                    p.Source @> jsonb_build_object('$type', @DockerVolumeType, 'PlatformId', r.PlatformId::text, 'VolumeName', r.VolumeName)
                    OR p.Source @> jsonb_build_object('$type', @DockerVolumeType, 'platformId', r.PlatformId::text, 'volumeName', r.VolumeName)
                 )
                 {{authorizationPredicate}}
            ),
            aggregate AS (
                SELECT
                    r.PlatformId,
                    r.VolumeName,
                    COUNT(p.Id)::int AS PolicyCount,
                    COUNT(p.Id) FILTER (WHERE p.Enabled)::int AS EnabledPolicyCount
                FROM requested r
                LEFT JOIN policies p
                  ON p.PlatformId = r.PlatformId
                 AND p.VolumeName = r.VolumeName
                GROUP BY r.PlatformId, r.VolumeName
            ),
            repository_readiness AS (
                SELECT
                    p.PlatformId,
                    p.VolumeName,
                    BOOL_OR(v.Status = @ReadyValidationStatus) AS HasReadyValidation
                FROM policies p
                LEFT JOIN BackupRepositoryValidations v
                  ON v.BackupRepositoryId = p.BackupRepositoryId
                 AND v.Location = @CoreLocation
                 AND v.PlatformId IS NULL
                GROUP BY p.PlatformId, p.VolumeName
            ),
            latest_runs AS (
                SELECT DISTINCT ON (p.PlatformId, p.VolumeName)
                    p.PlatformId,
                    p.VolumeName,
                    r.Id AS LastRunId,
                    r.Status AS LastRunStatus,
                    COALESCE(r.CompletedAt, r.QueuedAt) AS LastRunAt
                FROM policies p
                JOIN BackupRuns r ON r.BackupPolicyId = p.Id
                ORDER BY p.PlatformId, p.VolumeName, r.QueuedAt DESC, r.Id DESC
            ),
            latest_success AS (
                SELECT DISTINCT ON (p.PlatformId, p.VolumeName)
                    p.PlatformId,
                    p.VolumeName,
                    COALESCE(r.CompletedAt, r.QueuedAt) AS LastSuccessfulRunAt
                FROM policies p
                JOIN BackupRuns r ON r.BackupPolicyId = p.Id
                WHERE r.Status = ANY(@SuccessfulStatuses)
                  AND r.SnapshotAvailability = @AvailableSnapshot
                ORDER BY p.PlatformId, p.VolumeName, COALESCE(r.CompletedAt, r.QueuedAt) DESC, r.Id DESC
            )
            SELECT
                a.PlatformId AS PlatformId,
                a.VolumeName AS VolumeName,
                CASE
                    WHEN a.PolicyCount = 0 THEN @UnprotectedStatus
                    WHEN lr.LastRunStatus = @FailedRunStatus THEN @FailedStatus
                    WHEN a.EnabledPolicyCount = 0 THEN @WarningStatus
                    WHEN COALESCE(rr.HasReadyValidation, FALSE) = FALSE THEN @WarningStatus
                    WHEN lr.LastRunStatus = ANY(@WarningRunStatuses) THEN @WarningStatus
                    WHEN ls.LastSuccessfulRunAt IS NOT NULL THEN @ProtectedStatus
                    ELSE @WarningStatus
                END AS Status,
                a.PolicyCount AS PolicyCount,
                lr.LastRunId AS LastRunId,
                lr.LastRunStatus AS LastRunStatus,
                lr.LastRunAt AS LastRunAt,
                ls.LastSuccessfulRunAt AS LastSuccessfulRunAt,
                NULL::timestamp AS NextRunAt
            FROM aggregate a
            LEFT JOIN repository_readiness rr
              ON rr.PlatformId = a.PlatformId
             AND rr.VolumeName = a.VolumeName
            LEFT JOIN latest_runs lr
              ON lr.PlatformId = a.PlatformId
             AND lr.VolumeName = a.VolumeName
            LEFT JOIN latest_success ls
              ON ls.PlatformId = a.PlatformId
             AND ls.VolumeName = a.VolumeName
            ORDER BY a.VolumeName ASC
            """;

        var rows = await db.QueryAsync<VolumeBackupCoverageDto>(
            sql,
            new
            {
                PlatformIds = requested.Select(static volume => volume.PlatformId).ToArray(),
                VolumeNames = requested.Select(static volume => volume.VolumeName).ToArray(),
                UserId = userId ?? Guid.Empty,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission,
                DockerVolumeType = "DockerVolume",
                CoreLocation = EnumFormatter<BackupExecutionLocation>.GetValue(BackupExecutionLocation.Core),
                ReadyValidationStatus = EnumFormatter<BackupRepositoryValidationStatus>.GetValue(BackupRepositoryValidationStatus.Ready),
                AvailableSnapshot = EnumFormatter<BackupSnapshotAvailability>.GetValue(BackupSnapshotAvailability.Available),
                SuccessfulStatuses = new[]
                {
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Succeeded),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.SucceededWithWarnings)
                },
                WarningRunStatuses = new[]
                {
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.TimedOut),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Cancelled),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Interrupted),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.SucceededWithWarnings)
                },
                FailedRunStatus = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Failed),
                UnprotectedStatus = EnumFormatter<BackupCoverageStatus>.GetValue(BackupCoverageStatus.Unprotected),
                ProtectedStatus = EnumFormatter<BackupCoverageStatus>.GetValue(BackupCoverageStatus.Protected),
                WarningStatus = EnumFormatter<BackupCoverageStatus>.GetValue(BackupCoverageStatus.Warning),
                FailedStatus = EnumFormatter<BackupCoverageStatus>.GetValue(BackupCoverageStatus.Failed)
            },
            transaction: tx());

        return [.. rows.Select(static row => row.ToDomain())];
    }

    public async Task<IReadOnlyList<PlatformBackupSummary>> GetPlatformSummariesAsync(
        IReadOnlyCollection<Guid> platformIds,
        Guid? userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        var requested = platformIds
            .Where(static platformId => platformId != Guid.Empty)
            .Distinct()
            .ToArray();

        if (requested.Length == 0)
            return [];

        var authorizationCtes = userId.HasValue
            ? $", {AuthorizationSql.ActorScopeCte}, {AuthorizationSql.GlobalAccessCte}"
            : string.Empty;
        var authorizationPredicate = userId.HasValue
            ? $"AND {AuthorizationSql.ResourcePredicatePrefix}p.Id{AuthorizationSql.ResourcePredicateSuffix}"
            : string.Empty;

        string sql = $$"""
            WITH requested AS (
                SELECT unnest(@PlatformIds::uuid[]) AS PlatformId
            )
            {{authorizationCtes}},
            policies AS (
                SELECT
                    p.Id,
                    p.Enabled,
                    p.Source->>'$type' AS SourceType,
                    CASE p.Source->>'$type'
                        WHEN @DockerVolumeType THEN NULLIF(COALESCE(p.Source->>'PlatformId', p.Source->>'platformId'), '')::uuid
                        WHEN @StackType THEN sr.PlatformId
                        WHEN @DeploymentType THEN d.PlatformId
                        ELSE NULL
                    END AS PlatformId
                FROM BackupPolicies p
                LEFT JOIN Stacks s
                  ON p.Source->>'$type' = @StackType
                 AND s.Id = NULLIF(COALESCE(p.Source->>'StackId', p.Source->>'stackId'), '')::uuid
                LEFT JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
                LEFT JOIN Deployments d
                  ON p.Source->>'$type' = @DeploymentType
                 AND d.Id = NULLIF(COALESCE(p.Source->>'DeploymentId', p.Source->>'deploymentId'), '')::uuid
                WHERE p.ArchivedAt IS NULL
                  {{authorizationPredicate}}
            ),
            platform_policies AS (
                SELECT p.*
                FROM policies p
                JOIN requested r ON r.PlatformId = p.PlatformId
            ),
            latest_policy_runs AS (
                SELECT DISTINCT ON (p.Id)
                    p.Id AS PolicyId,
                    p.PlatformId,
                    r.Status,
                    COALESCE(r.CompletedAt, r.QueuedAt) AS RunAt
                FROM platform_policies p
                JOIN BackupRuns r ON r.BackupPolicyId = p.Id
                ORDER BY p.Id, r.QueuedAt DESC, r.Id DESC
            ),
            latest_platform_runs AS (
                SELECT DISTINCT ON (r.PlatformId)
                    r.PlatformId,
                    r.Status,
                    r.RunAt
                FROM latest_policy_runs r
                ORDER BY r.PlatformId, r.RunAt DESC, r.PolicyId DESC
            )
            SELECT
                requested.PlatformId,
                COUNT(p.Id)::int AS PolicyCount,
                COUNT(p.Id) FILTER (WHERE p.Enabled)::int AS EnabledPolicyCount,
                COUNT(p.Id) FILTER (WHERE p.SourceType = @DockerVolumeType)::int AS DockerVolumePolicyCount,
                COUNT(p.Id) FILTER (WHERE p.SourceType = @StackType)::int AS StackPolicyCount,
                COUNT(p.Id) FILTER (WHERE p.SourceType = @DeploymentType)::int AS DeploymentPolicyCount,
                COUNT(p.Id) FILTER (WHERE latest.Status = ANY(@AttentionStatuses))::int AS AttentionPolicyCount,
                latest_platform.Status AS LastRunStatus,
                latest_platform.RunAt AS LastRunAt
            FROM requested
            LEFT JOIN platform_policies p ON p.PlatformId = requested.PlatformId
            LEFT JOIN latest_policy_runs latest ON latest.PolicyId = p.Id
            LEFT JOIN latest_platform_runs latest_platform ON latest_platform.PlatformId = requested.PlatformId
            GROUP BY requested.PlatformId, latest_platform.Status, latest_platform.RunAt
            ORDER BY requested.PlatformId
            """;

        var rows = await db.QueryAsync<PlatformBackupSummaryDto>(
            sql,
            new
            {
                PlatformIds = requested,
                UserId = userId ?? Guid.Empty,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission,
                DockerVolumeType = "DockerVolume",
                StackType = "Stack",
                DeploymentType = "Deployment",
                AttentionStatuses = new[]
                {
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Failed),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.TimedOut),
                    EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Interrupted)
                }
            },
            transaction: tx());

        return [.. rows.Select(static row => row.ToDomain())];
    }

    public async Task<bool> TryMarkScheduledAsync(Guid id, DateTimeOffset scheduledMinuteUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET LastScheduledRunAt = @ScheduledMinuteUtc,
                UpdatedAt = @ScheduledMinuteUtc,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND ArchivedAt IS NULL
              AND Enabled
              AND Cron IS NOT NULL
              AND (LastScheduledRunAt IS NULL OR LastScheduledRunAt < @ScheduledMinuteUtc)
            """;

        var affected = await db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                ScheduledMinuteUtc = BackupMappers.ToUtcDateTime(scheduledMinuteUtc)
            },
            transaction: tx());

        return affected > 0;
    }

    public Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET ControlState = @ControlState,
                CurrentRunId = @RunId,
                ControlStartedAt = @ControlStartedAt,
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
                ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
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
                ControlStartedAt = NULL,
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
                ControlStartedAt = NULL,
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

    public async Task<IEnumerable<BackupPolicy>> GetStuckPoliciesAsync(CancellationToken cancellationToken = default)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BackupPolicies p
            LEFT JOIN BackupRuns r ON r.Id = p.CurrentRunId
            WHERE p.ArchivedAt IS NULL
              AND p.ControlState = @ControlState
              AND (
                  p.CurrentRunId IS NULL
                  OR r.Id IS NULL
                  OR r.Status <> ALL(@ActiveStatuses)
              )
            ORDER BY p.ControlStartedAt ASC
            """;

        var result = await db.QueryAsync<BackupPolicyDto>(
            sql,
            new
            {
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ActiveStatuses = ActiveBackupRunStatuses(),
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy)
            },
            transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateProcessingAsync(
        Guid id,
        ResourceControlState state,
        long? startedAt,
        long rowVersion,
        bool checkRowVersion,
        Guid? currentRunId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupPolicies
            SET ControlState = @State,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @StartedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND (@CheckRowVersion = false OR RowVersion = @RowVersion)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                CurrentRunId = currentRunId,
                StartedAt = startedAt,
                RowVersion = rowVersion,
                CheckRowVersion = checkRowVersion
            },
            transaction: tx());
    }

    private static string[] ActiveBackupRunStatuses()
        =>
        [
            EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
            EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Preparing),
            EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Running),
            EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.ApplyingRetention)
        ];
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
            p.Webhook AS PolicyWebhook,
            p.KeepLastSuccessful AS PolicyKeepLastSuccessful,
            p.TimeoutSeconds AS PolicyTimeoutSeconds,
            p.AlertOnFailure AS PolicyAlertOnFailure,
            p.RunAsActorId AS PolicyRunAsActorId,
            p.ControlState AS PolicyControlState,
            p.CurrentRunId AS PolicyCurrentRunId,
            p.ControlStartedAt AS PolicyControlStartedAt,
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
            br.ControlState AS RepositoryControlState,
            br.CurrentRunId AS RepositoryCurrentRunId,
            br.ControlStartedAt AS RepositoryControlStartedAt,
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
            4 => new BackupRunQueueResult(BackupRunQueueResultStatus.AlreadyScheduled, null),
            _ => new BackupRunQueueResult(BackupRunQueueResultStatus.ActiveRunExists, null)
        };
    }

    public async Task<BackupRunQueueResult> QueueScheduledAsync(
        Guid policyId,
        Guid runId,
        DateTimeOffset scheduledMinuteUtc,
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
                    p.Enabled,
                    p.Cron,
                    p.LastScheduledRunAt,
                    p.ArchivedAt,
                    r.Type AS RepositoryTypeSnapshot
                FROM BackupPolicies p
                JOIN BackupRepositories r ON r.Id = p.BackupRepositoryId
                WHERE p.Id = @PolicyId
                LIMIT 1
            ),
            marked AS (
                UPDATE BackupPolicies p
                SET LastScheduledRunAt = @ScheduledMinuteUtc,
                    UpdatedAt = @ScheduledMinuteUtc,
                    RowVersion = p.RowVersion + 1
                FROM policy target
                WHERE p.Id = target.BackupPolicyId
                  AND target.ArchivedAt IS NULL
                  AND target.Enabled
                  AND target.Cron IS NOT NULL
                  AND (target.LastScheduledRunAt IS NULL OR target.LastScheduledRunAt < @ScheduledMinuteUtc)
                RETURNING p.Id
            ),
            inserted AS (
                INSERT INTO BackupRuns (
                    Id, BackupPolicyId, BackupRepositoryId, PolicyNameSnapshot, SourceSnapshot, RepositoryTypeSnapshot,
                    Trigger, TriggerSourceId, Status, ResticSnapshotId, ParentSnapshotId, SnapshotAvailability,
                    FilesProcessed, BytesProcessed, BytesAdded, Warnings, QueuedAt, StartedAt, CompletedAt,
                    ExitCode, ErrorCode, ErrorMessage, TriggeredByActorId)
                SELECT
                    @RunId, p.BackupPolicyId, p.BackupRepositoryId, p.PolicyNameSnapshot, p.SourceSnapshot, p.RepositoryTypeSnapshot,
                    @Trigger, NULL, @Status, NULL, NULL, @SnapshotAvailability,
                    NULL, NULL, NULL, @Warnings::jsonb, @ScheduledMinuteUtc, NULL, NULL,
                    NULL, NULL, NULL, p.RunAsActorId
                FROM policy p
                JOIN marked m ON m.Id = p.BackupPolicyId
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
                    WHEN NOT EXISTS (SELECT 1 FROM marked) THEN 4
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
                ScheduledMinuteUtc = BackupMappers.ToUtcDateTime(scheduledMinuteUtc),
                Trigger = EnumFormatter<BackupRunTrigger>.GetValue(BackupRunTrigger.Schedule),
                Status = EnumFormatter<BackupRunStatus>.GetValue(BackupRunStatus.Queued),
                SnapshotAvailability = EnumFormatter<BackupSnapshotAvailability>.GetValue(BackupSnapshotAvailability.Pending),
                Warnings = BackupMappers.SerializeWarnings([])
            },
            transaction: tx());

        return result.ResultStatus switch
        {
            0 => new BackupRunQueueResult(BackupRunQueueResultStatus.Queued, result.ToDomain()),
            1 => new BackupRunQueueResult(BackupRunQueueResultStatus.PolicyNotFound, null),
            2 => new BackupRunQueueResult(BackupRunQueueResultStatus.PolicyArchived, null),
            4 => new BackupRunQueueResult(BackupRunQueueResultStatus.AlreadyScheduled, null),
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

    public async Task<BackupRunFinishOutcome> FinishRunAndMarkPolicyIdleAsync(
        BackupRun run,
        Guid policyId,
        bool successful,
        DateTimeOffset completedAt,
        CancellationToken cancellationToken)
    {
        string sql = $$"""
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
                UPDATE BackupPolicies p
                SET ControlState = @IdleControlState,
                    CurrentRunId = NULL,
                    ControlStartedAt = NULL,
                    FirstSuccessfulRunAt = CASE
                        WHEN @Successful AND EXISTS (SELECT 1 FROM updated_run) THEN COALESCE(FirstSuccessfulRunAt, @CompletedAt)
                        ELSE FirstSuccessfulRunAt
                    END,
                    UpdatedAt = @CompletedAt,
                    RowVersion = RowVersion + 1
                WHERE p.Id = @PolicyId
                  AND CurrentRunId = @Id
                RETURNING p.*
            )
            SELECT
                CASE
                    WHEN EXISTS (SELECT 1 FROM updated_run) THEN 0
                    ELSE 1
                END AS ResultStatus,
                p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM (SELECT 1) s
            LEFT JOIN updated_policy p ON TRUE
            """;

        var result = await db.QuerySingleAsync<BackupRunFinishDto>(
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
                IdleControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle),
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.BackupPolicy)
            },
            transaction: tx());

        return result.ToDomain();
    }

    public async Task<BackupRun?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM BackupRuns WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<BackupRunDto>(sql, new { Id = id }, transaction: tx());
        if (result is null)
            return null;

        var run = result.ToDomain();
        await AssignItemsAsync([run], cancellationToken);
        return run;
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
                    ControlStartedAt = @ControlStartedAt,
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
                ControlStartedAt = startedAt.ToUniversalTime().ToUnixTimeSeconds(),
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
        var runs = result.ToDomain().ToArray();
        await AssignItemsAsync(runs, cancellationToken);
        return runs;
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
        var runs = result.ToDomain().ToArray();
        await AssignItemsAsync(runs, cancellationToken);
        return runs;
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

    public async Task<BackupRun?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken)
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
            RETURNING *
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRunDto>(
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

        if (result is null)
            return null;

        var run = result.ToDomain();
        await AssignItemsAsync([run], cancellationToken);
        return run;
    }

    private async Task AssignItemsAsync(IReadOnlyCollection<BackupRun> runs, CancellationToken cancellationToken)
    {
        if (runs.Count == 0)
            return;

        const string sql = """
            SELECT *
            FROM BackupRunItems
            WHERE BackupRunId = ANY(@BackupRunIds)
            ORDER BY VolumeName ASC, Id ASC
            """;

        var rows = await db.QueryAsync<BackupRunItemDto>(
            sql,
            new { BackupRunIds = runs.Select(static run => run.Id).ToArray() },
            transaction: tx());

        var itemsByRun = rows
            .Select(static row => row.ToDomain())
            .GroupBy(static item => item.BackupRunId)
            .ToDictionary(static group => group.Key, static group => (IReadOnlyList<BackupRunItem>)[.. group]);

        foreach (var run in runs)
        {
            run.AssignItems(itemsByRun.TryGetValue(run.Id, out var items) ? items : []);
        }
    }

}

internal sealed class BackupRunItemRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRunItemRepository
{
    public Task<int> AddRangeAsync(IReadOnlyCollection<BackupRunItem> items, CancellationToken cancellationToken)
    {
        if (items.Count == 0)
            return Task.FromResult(0);

        var batch = items.ToArray();
        const string sql = """
            INSERT INTO BackupRunItems (
                Id, BackupRunId, PlatformId, VolumeName, Status, ResticSnapshotId, ParentSnapshotId,
                FilesProcessed, BytesProcessed, BytesAdded, StartedAt, CompletedAt, ExitCode,
                ErrorCode, ErrorMessage, CreatedAt, UpdatedAt)
            SELECT
                Id, BackupRunId, PlatformId, VolumeName, Status, ResticSnapshotId, ParentSnapshotId,
                FilesProcessed, BytesProcessed, BytesAdded, StartedAt, CompletedAt, ExitCode,
                ErrorCode, ErrorMessage, CreatedAt, UpdatedAt
            FROM unnest(
                @Ids::uuid[],
                @BackupRunIds::uuid[],
                @PlatformIds::uuid[],
                @VolumeNames::text[],
                @Statuses::text[],
                @ResticSnapshotIds::text[],
                @ParentSnapshotIds::text[],
                @FilesProcessed::bigint[],
                @BytesProcessed::bigint[],
                @BytesAdded::bigint[],
                @StartedAts::timestamp[],
                @CompletedAts::timestamp[],
                @ExitCodes::integer[],
                @ErrorCodes::text[],
                @ErrorMessages::text[],
                @CreatedAts::timestamp[],
                @UpdatedAts::timestamp[])
                AS items(
                    Id, BackupRunId, PlatformId, VolumeName, Status, ResticSnapshotId, ParentSnapshotId,
                    FilesProcessed, BytesProcessed, BytesAdded, StartedAt, CompletedAt, ExitCode,
                    ErrorCode, ErrorMessage, CreatedAt, UpdatedAt)
            ON CONFLICT (Id) DO NOTHING
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = batch.Select(static item => item.Id).ToArray(),
                BackupRunIds = batch.Select(static item => item.BackupRunId).ToArray(),
                PlatformIds = batch.Select(static item => item.PlatformId).ToArray(),
                VolumeNames = batch.Select(static item => item.VolumeName).ToArray(),
                Statuses = batch.Select(static item => EnumFormatter<BackupRunItemStatus>.GetValue(item.Status)).ToArray(),
                ResticSnapshotIds = batch.Select(static item => item.ResticSnapshotId).ToArray(),
                ParentSnapshotIds = batch.Select(static item => item.ParentSnapshotId).ToArray(),
                FilesProcessed = batch.Select(static item => item.FilesProcessed).ToArray(),
                BytesProcessed = batch.Select(static item => item.BytesProcessed).ToArray(),
                BytesAdded = batch.Select(static item => item.BytesAdded).ToArray(),
                StartedAts = batch.Select(static item => BackupMappers.ToUtcDateTime(item.StartedAt)).ToArray(),
                CompletedAts = batch.Select(static item => BackupMappers.ToUtcDateTime(item.CompletedAt)).ToArray(),
                ExitCodes = batch.Select(static item => item.ExitCode).ToArray(),
                ErrorCodes = batch.Select(static item => item.ErrorCode).ToArray(),
                ErrorMessages = batch.Select(static item => item.ErrorMessage).ToArray(),
                CreatedAts = batch.Select(static item => BackupMappers.ToUtcDateTime(item.CreatedAt)).ToArray(),
                UpdatedAts = batch.Select(static item => BackupMappers.ToUtcDateTime(item.UpdatedAt)).ToArray()
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(BackupRunItem item, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRunItems
            SET Status = @Status,
                ResticSnapshotId = @ResticSnapshotId,
                ParentSnapshotId = @ParentSnapshotId,
                FilesProcessed = @FilesProcessed,
                BytesProcessed = @BytesProcessed,
                BytesAdded = @BytesAdded,
                StartedAt = @StartedAt,
                CompletedAt = @CompletedAt,
                ExitCode = @ExitCode,
                ErrorCode = @ErrorCode,
                ErrorMessage = @ErrorMessage,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                item.Id,
                Status = EnumFormatter<BackupRunItemStatus>.GetValue(item.Status),
                item.ResticSnapshotId,
                item.ParentSnapshotId,
                item.FilesProcessed,
                item.BytesProcessed,
                item.BytesAdded,
                StartedAt = BackupMappers.ToUtcDateTime(item.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(item.CompletedAt),
                item.ExitCode,
                item.ErrorCode,
                item.ErrorMessage,
                UpdatedAt = BackupMappers.ToUtcDateTime(item.UpdatedAt)
            },
            transaction: tx());
    }

    public async Task<IReadOnlyList<BackupRunItem>> GetByRunAsync(Guid backupRunId, CancellationToken cancellationToken)
        => await GetByRunIdsAsync([backupRunId], cancellationToken);

    public async Task<IReadOnlyList<BackupRunItem>> GetByRunIdsAsync(IReadOnlyCollection<Guid> backupRunIds, CancellationToken cancellationToken)
    {
        if (backupRunIds.Count == 0)
            return [];

        const string sql = """
            SELECT *
            FROM BackupRunItems
            WHERE BackupRunId = ANY(@BackupRunIds)
            ORDER BY BackupRunId ASC, VolumeName ASC, Id ASC
            """;

        var rows = await db.QueryAsync<BackupRunItemDto>(
            sql,
            new { BackupRunIds = backupRunIds.ToArray() },
            transaction: tx());
        return [.. rows.Select(static row => row.ToDomain())];
    }

    public Task<int> CancelPendingOrRunningAsync(Guid backupRunId, DateTimeOffset cancelledAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BackupRunItems
            SET Status = @CancelledStatus,
                CompletedAt = @CancelledAt,
                ErrorCode = @ErrorCode,
                ErrorMessage = @ErrorMessage,
                UpdatedAt = @CancelledAt
            WHERE BackupRunId = @BackupRunId
              AND Status = ANY(@ActiveStatuses)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                BackupRunId = backupRunId,
                CancelledAt = BackupMappers.ToUtcDateTime(cancelledAt),
                CancelledStatus = EnumFormatter<BackupRunItemStatus>.GetValue(BackupRunItemStatus.Cancelled),
                ErrorCode = "backup.cancelled",
                ErrorMessage = "Backup run cancelled.",
                ActiveStatuses = new[]
                {
                    EnumFormatter<BackupRunItemStatus>.GetValue(BackupRunItemStatus.Pending),
                    EnumFormatter<BackupRunItemStatus>.GetValue(BackupRunItemStatus.Running)
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
    private const string RestoreExecutionPlanSelect = """
            rr.Id AS RestoreRunId,
            rr.BackupRunId AS RestoreBackupRunId,
            rr.BackupRepositoryId AS RestoreBackupRepositoryId,
            rr.Status AS RestoreStatus,
            rr.TargetPlatformId AS RestoreTargetPlatformId,
            rr.TargetVolumeName AS RestoreTargetVolumeName,
            rr.OverwriteExisting AS RestoreOverwriteExisting,
            rr.TargetVolumeCreatedByCitadel AS RestoreTargetVolumeCreatedByCitadel,
            rr.AffectedContainers AS RestoreAffectedContainers,
            rr.Warnings AS RestoreWarnings,
            rr.QueuedAt AS RestoreQueuedAt,
            rr.StartedAt AS RestoreStartedAt,
            rr.CompletedAt AS RestoreCompletedAt,
            rr.ExitCode AS RestoreExitCode,
            rr.ErrorCode AS RestoreErrorCode,
            rr.ErrorMessage AS RestoreErrorMessage,
            rr.TriggeredByActorId AS RestoreTriggeredByActorId,
            brn.Id AS RunId,
            brn.BackupPolicyId AS RunBackupPolicyId,
            brn.BackupRepositoryId AS RunBackupRepositoryId,
            brn.PolicyNameSnapshot AS RunPolicyNameSnapshot,
            brn.SourceSnapshot AS RunSourceSnapshot,
            brn.RepositoryTypeSnapshot AS RunRepositoryTypeSnapshot,
            brn.Trigger AS RunTrigger,
            brn.TriggerSourceId AS RunTriggerSourceId,
            brn.Status AS RunStatus,
            brn.ResticSnapshotId AS RunResticSnapshotId,
            brn.ParentSnapshotId AS RunParentSnapshotId,
            brn.SnapshotAvailability AS RunSnapshotAvailability,
            brn.FilesProcessed AS RunFilesProcessed,
            brn.BytesProcessed AS RunBytesProcessed,
            brn.BytesAdded AS RunBytesAdded,
            brn.Warnings AS RunWarnings,
            brn.QueuedAt AS RunQueuedAt,
            brn.StartedAt AS RunStartedAt,
            brn.CompletedAt AS RunCompletedAt,
            brn.ExitCode AS RunExitCode,
            brn.ErrorCode AS RunErrorCode,
            brn.ErrorMessage AS RunErrorMessage,
            brn.TriggeredByActorId AS RunTriggeredByActorId,
            repo.Id AS RepositoryId,
            repo.Name AS RepositoryName,
            repo.NormalizedName AS RepositoryNormalizedName,
            repo.Description AS RepositoryDescription,
            repo.Type AS RepositoryType,
            repo.Spec AS RepositorySpec,
            repo.PasswordSecretId AS RepositoryPasswordSecretId,
            repo.Status AS RepositoryStatus,
            repo.ControlState AS RepositoryControlState,
            repo.CurrentRunId AS RepositoryCurrentRunId,
            repo.ControlStartedAt AS RepositoryControlStartedAt,
            repo.LastPrunedAt AS RepositoryLastPrunedAt,
            repo.LastCheckedAt AS RepositoryLastCheckedAt,
            repo.CreatedByActorId AS RepositoryCreatedByActorId,
            repo.CreatedAt AS RepositoryCreatedAt,
            repo.UpdatedAt AS RepositoryUpdatedAt,
            repo.ArchivedAt AS RepositoryArchivedAt,
            repo.RowVersion AS RepositoryRowVersion
        """;

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

    public async Task<BackupRestoreRunExecutionPlan?> GetExecutionPlanAsync(Guid id, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT
                {{RestoreExecutionPlanSelect}}
            FROM BackupRestoreRuns rr
            JOIN BackupRuns brn ON brn.Id = rr.BackupRunId
            JOIN BackupRepositories repo ON repo.Id = rr.BackupRepositoryId
            WHERE rr.Id = @Id
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRestoreRunExecutionPlanDto>(
            sql,
            new { Id = id },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<BackupRestoreRunExecutionPlan?> TryClaimExecutionPlanAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken)
    {
        string sql = $$"""
            WITH claimed AS (
                UPDATE BackupRestoreRuns
                SET Status = @PreparingStatus,
                    StartedAt = COALESCE(StartedAt, @StartedAt)
                WHERE Id = @Id
                  AND Status = @QueuedStatus
                RETURNING *
            )
            SELECT
                {{RestoreExecutionPlanSelect}}
            FROM claimed rr
            JOIN BackupRuns brn ON brn.Id = rr.BackupRunId
            JOIN BackupRepositories repo ON repo.Id = rr.BackupRepositoryId
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRestoreRunExecutionPlanDto>(
            sql,
            new
            {
                Id = id,
                StartedAt = BackupMappers.ToUtcDateTime(startedAt),
                QueuedStatus = EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Queued),
                PreparingStatus = EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Preparing)
            },
            transaction: tx());
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

    public async Task<IEnumerable<BackupRestoreRun>> GetByPolicyAsync(Guid policyId, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT rr.*
            FROM BackupRestoreRuns rr
            JOIN BackupRuns br ON br.Id = rr.BackupRunId
            WHERE br.BackupPolicyId = @PolicyId
            ORDER BY rr.QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<BackupRestoreRunDto>(
            sql,
            new { PolicyId = policyId, Limit = Math.Clamp(limit, 1, 200) },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<IReadOnlyList<Guid>> GetQueuedIdsAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id
            FROM BackupRestoreRuns
            WHERE Status = @Status
            ORDER BY QueuedAt ASC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<Guid>(
            sql,
            new
            {
                Status = EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Queued),
                Limit = Math.Clamp(limit, 1, 100)
            },
            transaction: tx());
        return [.. result];
    }

    public async Task<BackupRestoreRunFinishResult> FinishRunAsync(BackupRestoreRun run, DateTimeOffset completedAt, CancellationToken cancellationToken)
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
              AND (@Status = @CancelledStatus OR Status <> @CancelledStatus)
            """;

        var rows = await db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                Status = EnumFormatter<BackupRestoreStatus>.GetValue(run.Status),
                CancelledStatus = EnumFormatter<BackupRestoreStatus>.GetValue(BackupRestoreStatus.Cancelled),
                run.TargetVolumeCreatedByCitadel,
                AffectedContainers = BackupMappers.SerializeAffectedContainers(run.AffectedContainers),
                Warnings = BackupMappers.SerializeWarnings(run.Warnings),
                StartedAt = BackupMappers.ToUtcDateTime(run.StartedAt),
                CompletedAt = BackupMappers.ToUtcDateTime(completedAt),
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage
            },
            transaction: tx());

        return rows > 0 ? BackupRestoreRunFinishResult.Completed : BackupRestoreRunFinishResult.AlreadyCancelled;
    }

    public async Task<BackupRestoreRunWithPolicy?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH cancelled AS (
                UPDATE BackupRestoreRuns
                SET Status = @CancelledStatus,
                    CompletedAt = @CancelledAt,
                    ErrorCode = @ErrorCode,
                    ErrorMessage = @Reason
                WHERE Id = @Id
                  AND Status = ANY(@ActiveStatuses)
                RETURNING *
            )
            SELECT cancelled.*, br.BackupPolicyId
            FROM cancelled
            JOIN BackupRuns br ON br.Id = cancelled.BackupRunId
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BackupRestoreRunWithPolicyDto>(
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
        return result?.ToDomain();
    }

}

internal sealed class BackupRestoreRunLogRepository(IDbConnection db, Func<IDbTransaction> tx) : IBackupRestoreRunLogRepository
{
    public Task<int> AddRangeAsync(IReadOnlyCollection<BackupRestoreRunLogEntry> entries, CancellationToken cancellationToken)
    {
        if (entries.Count == 0)
            return Task.FromResult(0);

        var batch = entries.ToArray();
        const string sql = """
            INSERT INTO BackupRestoreRunLogs (Id, BackupRestoreRunId, CreatedAt, Stream, Message)
            SELECT Id, BackupRestoreRunId, CreatedAt, Stream, Message
            FROM unnest(
                @Ids::uuid[],
                @BackupRestoreRunIds::uuid[],
                @CreatedAts::timestamp[],
                @Streams::text[],
                @Messages::text[])
                AS logs(Id, BackupRestoreRunId, CreatedAt, Stream, Message)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = batch.Select(static entry => entry.Id).ToArray(),
                BackupRestoreRunIds = batch.Select(static entry => entry.BackupRestoreRunId).ToArray(),
                CreatedAts = batch.Select(static entry => BackupMappers.ToUtcDateTime(entry.CreatedAt)).ToArray(),
                Streams = batch.Select(static entry => entry.Stream).ToArray(),
                Messages = batch.Select(static entry => entry.Message).ToArray()
            },
            transaction: tx());
    }

    public async Task<IReadOnlyList<BackupRestoreRunLogEntry>> GetByRunAsync(Guid restoreRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM BackupRestoreRunLogs
            WHERE BackupRestoreRunId = @RestoreRunId
            ORDER BY CreatedAt ASC, Id ASC
            """;

        var result = await db.QueryAsync<BackupRestoreRunLogDto>(sql, new { RestoreRunId = restoreRunId }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<BackupRestoreRunLogs> GetByRunWithRunStateAsync(Guid restoreRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH run_state AS (
                SELECT EXISTS(
                    SELECT 1
                    FROM BackupRestoreRuns
                    WHERE Id = @RestoreRunId
                ) AS RunExists
            )
            SELECT
                rs.RunExists AS RunExists,
                l.Id AS LogId,
                l.BackupRestoreRunId AS LogBackupRestoreRunId,
                l.CreatedAt AS LogCreatedAt,
                l.Stream AS LogStream,
                l.Message AS LogMessage
            FROM run_state rs
            LEFT JOIN BackupRestoreRunLogs l
              ON rs.RunExists
             AND l.BackupRestoreRunId = @RestoreRunId
            ORDER BY l.CreatedAt ASC NULLS LAST, l.Id ASC NULLS LAST
            """;

        var rows = await db.QueryAsync<BackupRestoreRunLogWithRunStateDto>(
            sql,
            new { RestoreRunId = restoreRunId },
            transaction: tx());

        var list = rows as BackupRestoreRunLogWithRunStateDto[] ?? [.. rows];
        if (list.Length == 0 || !list[0].RunExists)
            return new BackupRestoreRunLogs(false, []);

        var logs = list
            .Where(static row => row.LogId.HasValue)
            .Select(static row => new BackupRestoreRunLogEntry(
                row.LogId!.Value,
                row.LogBackupRestoreRunId!.Value,
                BackupMappers.ToOffset(row.LogCreatedAt)!.Value,
                row.LogStream ?? string.Empty,
                row.LogMessage ?? string.Empty))
            .ToArray();

        return new BackupRestoreRunLogs(true, logs);
    }
}
