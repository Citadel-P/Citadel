using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Domain.Entities.Tags;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class BuildProjectRepository(IDbConnection db, Func<IDbTransaction> tx) : IBuildProjectRepository
{
    public async Task<int> AddAsync(
        BuildProject project,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? tagCreatedByActorId = null)
    {
        project.Validate();

        string sql = ResourceTagSql.InputTagsCte + """
            inserted_project AS (
                INSERT INTO BuildProjects (
                    Id, Name, NormalizedName, Description, Enabled, GitRepositoryId, Branch,
                    ContextPath, DockerfilePath, Target, BuildArgs, BuildSecrets, PlatformId,
                    RegistryId, ImageRepository, TagTemplates, TimeoutSeconds, RetentionRunCount,
                    CurrentRunId, ControlState, ControlStartedAt, CreatedByActorId, CreatedAt, UpdatedAt, ArchivedAt, RowVersion)
                SELECT
                    @Id, @Name, @NormalizedName, @Description, @Enabled, @GitRepositoryId, @Branch,
                    @ContextPath, @DockerfilePath, @Target, @BuildArgs::jsonb, @BuildSecrets::jsonb, @PlatformId,
                    @RegistryId, @ImageRepository, @TagTemplates::jsonb, @TimeoutSeconds, @RetentionRunCount,
                    @CurrentRunId, @ControlState, @ControlStartedAt, @CreatedByActorId, @CreatedAt, @UpdatedAt, @ArchivedAt, @RowVersion
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_project", "p") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_project", "inserted_project", "inserted_tags");

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(
            sql,
            new
            {
                project.Id,
                project.Name,
                project.NormalizedName,
                project.Description,
                project.Enabled,
                project.GitRepositoryId,
                project.Branch,
                project.ContextPath,
                project.DockerfilePath,
                project.Target,
                BuildArgs = BuildMappers.SerializeBuildArgs(project.BuildArgs),
                BuildSecrets = BuildMappers.SerializeBuildSecrets(project.BuildSecrets),
                project.PlatformId,
                project.RegistryId,
                project.ImageRepository,
                TagTemplates = BuildMappers.SerializeStringList(project.TagTemplates),
                project.TimeoutSeconds,
                project.RetentionRunCount,
                project.CurrentRunId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(project.ControlState),
                project.ControlStartedAt,
                project.CreatedByActorId,
                CreatedAt = project.CreatedAt.UtcDateTime,
                UpdatedAt = project.UpdatedAt.UtcDateTime,
                ArchivedAt = project.ArchivedAt?.UtcDateTime,
                project.RowVersion,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Build),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length,
                TagCreatedByActorId = tagCreatedByActorId ?? project.CreatedByActorId
            },
            transaction: tx());

        project.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public Task<int> UpdateAsync(BuildProject project, CancellationToken cancellationToken)
    {
        project.Validate();

        const string sql = """
            UPDATE BuildProjects
            SET Name = @Name,
                NormalizedName = @NormalizedName,
                Description = @Description,
                Enabled = @Enabled,
                GitRepositoryId = @GitRepositoryId,
                Branch = @Branch,
                ContextPath = @ContextPath,
                DockerfilePath = @DockerfilePath,
                Target = @Target,
                BuildArgs = @BuildArgs::jsonb,
                BuildSecrets = @BuildSecrets::jsonb,
                PlatformId = @PlatformId,
                RegistryId = @RegistryId,
                ImageRepository = @ImageRepository,
                TagTemplates = @TagTemplates::jsonb,
                TimeoutSeconds = @TimeoutSeconds,
                RetentionRunCount = @RetentionRunCount,
                CurrentRunId = @CurrentRunId,
                ControlState = @ControlState,
                ControlStartedAt = @ControlStartedAt,
                UpdatedAt = @UpdatedAt,
                ArchivedAt = @ArchivedAt,
                RowVersion = @RowVersion
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            ToParameters(project),
            transaction: tx());
    }

    public Task<int> ArchiveAsync(Guid id, DateTimeOffset archivedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildProjects
            SET Enabled = FALSE,
                ArchivedAt = COALESCE(ArchivedAt, @ArchivedAt),
                UpdatedAt = @ArchivedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND CurrentRunId IS NULL
              AND ControlState = @IdleControlState
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                ArchivedAt = archivedAt.UtcDateTime,
                IdleControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle)
            },
            transaction: tx());
    }

    public async Task<BuildProject?> GetAsync(Guid id, CancellationToken cancellationToken, bool includeArchived = false)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BuildProjects p
            WHERE p.Id = @Id
              AND (@IncludeArchived OR p.ArchivedAt IS NULL)
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<BuildProjectDto>(
            sql,
            new
            {
                Id = id,
                IncludeArchived = includeArchived,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Build)
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BuildProject>> GetAllAsync(
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        bool includeArchived = false)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BuildProjects p
            WHERE (@IncludeArchived OR p.ArchivedAt IS NULL)
              AND {{ResourceTagSql.FilterPredicate("p")}}
            ORDER BY p.Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<BuildProjectDto>(
            sql,
            new
            {
                IncludeArchived = includeArchived,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Build),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());
        return result.Select(static x => x.ToDomain());
    }

    public async Task<IEnumerable<BuildProject>> GetAuthorizedAsync(
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
            FROM BuildProjects p
            WHERE p.ArchivedAt IS NULL
              AND {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}}
              AND {{ResourceTagSql.FilterPredicate("p")}}
            ORDER BY p.Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<BuildProjectDto>(
            sql,
            new
            {
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Build),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());
        return result.Select(static x => x.ToDomain());
    }

    public Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1 FROM BuildProjects
                WHERE NormalizedName = @NormalizedName AND ArchivedAt IS NULL)
            """;
        return db.ExecuteScalarAsync<bool>(sql, new { NormalizedName = normalizedName }, transaction: tx());
    }

    public Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1 FROM BuildProjects
                WHERE NormalizedName = @NormalizedName AND Id <> @Id AND ArchivedAt IS NULL)
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
                FROM BuildProjects p
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
            UPDATE BuildProjects
            SET ControlState = @ControlState,
                CurrentRunId = @RunId,
                ControlStartedAt = @ControlStartedAt,
                UpdatedAt = @UpdatedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND ArchivedAt IS NULL
              AND CurrentRunId IS NULL
              AND ControlState = @IdleControlState
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                RunId = runId,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                IdleControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Idle),
                ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                UpdatedAt = DateTime.UtcNow
            },
            transaction: tx());
    }

    public Task<int> MarkIdleAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildProjects
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

    public async Task<IEnumerable<BuildProject>> GetStuckProjectsAsync(int graceSeconds = 300, CancellationToken cancellationToken = default)
    {
        string sql = $$"""
            SELECT p.*,
                {{ResourceTagSql.TagAggregate("p")}}
            FROM BuildProjects p
            LEFT JOIN BuildRuns r ON r.Id = p.CurrentRunId
            WHERE p.ControlState = @ControlState
              AND p.ArchivedAt IS NULL
              AND (
                  p.CurrentRunId IS NULL
                  OR r.Id IS NULL
                  OR r.Status <> ALL(@ActiveStatuses)
                  OR (
                      r.Status = ANY(@TimedActiveStatuses)
                      AND r.StartedAt IS NOT NULL
                      AND EXTRACT(EPOCH FROM r.StartedAt) < @NowEpoch - (r.TimeoutSeconds + @GraceSeconds)
                  )
              )
            ORDER BY p.ControlStartedAt ASC
            """;

        var queuedStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued);
        var preparingStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Preparing);
        var runningStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Running);

        var result = await db.QueryAsync<BuildProjectDto>(
            sql,
            new
            {
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ActiveStatuses = new[] { queuedStatus, preparingStatus, runningStatus },
                TimedActiveStatuses = new[] { preparingStatus, runningStatus },
                NowEpoch = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                GraceSeconds = graceSeconds,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Build)
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
            UPDATE BuildProjects
            SET ControlState = @State,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @StartedAt,
                UpdatedAt = @UpdatedAt,
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
                UpdatedAt = DateTime.UtcNow,
                RowVersion = rowVersion,
                CheckRowVersion = checkRowVersion
            },
            transaction: tx());
    }

    private static BuildProjectParameters ToParameters(BuildProject project)
        => new
        (
            project.Id,
            project.Name,
            project.NormalizedName,
            project.Description,
            project.Enabled,
            project.GitRepositoryId,
            project.Branch,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            BuildMappers.SerializeBuildArgs(project.BuildArgs),
            BuildMappers.SerializeBuildSecrets(project.BuildSecrets),
            project.PlatformId,
            project.RegistryId,
            project.ImageRepository,
            BuildMappers.SerializeStringList(project.TagTemplates),
            project.TimeoutSeconds,
            project.RetentionRunCount,
            project.CurrentRunId,
            EnumFormatter<ResourceControlState>.GetValue(project.ControlState),
            project.ControlStartedAt,
            project.UpdatedAt.UtcDateTime,
            project.ArchivedAt?.UtcDateTime,
            project.RowVersion
        );
}

internal sealed class BuildRunRepository(IDbConnection db, Func<IDbTransaction> tx) : IBuildRunRepository
{
    public Task<int> AddAsync(BuildRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO BuildRuns (
                Id, BuildProjectId, ProjectNameSnapshot, GitRepositoryId, GitRepositoryNameSnapshot, Branch,
                ResolvedCommitSha, ContextPath, DockerfilePath, Target, BuildArgsSnapshot,
                BuildSecretIdsSnapshot, PlatformSnapshot, RegistrySnapshot, ImageRepository,
                TagTemplatesSnapshot, ImageReferences, Trigger, TriggerSourceId, Status, ImageDigest,
                TimeoutSeconds, QueuedAt, StartedAt, CompletedAt, ExitCode, ErrorCode, ErrorMessage,
                TriggeredByActorId)
            VALUES (
                @Id, @BuildProjectId, @ProjectNameSnapshot, @GitRepositoryId, @GitRepositoryNameSnapshot, @Branch,
                @ResolvedCommitSha, @ContextPath, @DockerfilePath, @Target, @BuildArgsSnapshot::jsonb,
                @BuildSecretIdsSnapshot::jsonb, @PlatformSnapshot::jsonb, @RegistrySnapshot::jsonb, @ImageRepository,
                @TagTemplatesSnapshot::jsonb, @ImageReferences::jsonb, @Trigger, @TriggerSourceId, @Status, @ImageDigest,
                @TimeoutSeconds, @QueuedAt, @StartedAt, @CompletedAt, @ExitCode, @ErrorCode, @ErrorMessage,
                @TriggeredByActorId)
            """;

        return db.ExecuteAsync(sql, ToParameters(run), transaction: tx());
    }

    public Task<int> UpdateAsync(BuildRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildRuns
            SET ResolvedCommitSha = @ResolvedCommitSha,
                ImageReferences = @ImageReferences::jsonb,
                Status = @Status,
                ImageDigest = @ImageDigest,
                StartedAt = @StartedAt,
                CompletedAt = @CompletedAt,
                ExitCode = @ExitCode,
                ErrorCode = @ErrorCode,
                ErrorMessage = @ErrorMessage
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(sql, ToParameters(run), transaction: tx());
    }

    public async Task<BuildRun?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM BuildRuns WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<BuildRunDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<BuildRun>> GetByProjectAsync(Guid projectId, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM BuildRuns
            WHERE BuildProjectId = @ProjectId
            ORDER BY QueuedAt DESC, Id DESC
            LIMIT @Limit
            """;
        var result = await db.QueryAsync<BuildRunDto>(sql, new { ProjectId = projectId, Limit = limit }, transaction: tx());
        return result.Select(static x => x.ToDomain());
    }

    public async Task<IEnumerable<BuildRun>> GetPagedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM BuildRuns ORDER BY QueuedAt DESC, Id DESC LIMIT @Limit";
        var result = await db.QueryAsync<BuildRunDto>(sql, new { Limit = limit }, transaction: tx());
        return result.Select(static x => x.ToDomain());
    }

    public async Task<BuildRun?> GetLatestByProjectAsync(Guid projectId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM BuildRuns
            WHERE BuildProjectId = @ProjectId
            ORDER BY QueuedAt DESC, Id DESC
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<BuildRunDto>(sql, new { ProjectId = projectId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IReadOnlyDictionary<Guid, BuildRun>> GetLatestByProjectsAsync(
        IReadOnlyCollection<Guid> projectIds,
        CancellationToken cancellationToken)
    {
        if (projectIds.Count == 0)
            return new Dictionary<Guid, BuildRun>();

        const string sql = """
            SELECT DISTINCT ON (BuildProjectId) *
            FROM BuildRuns
            WHERE BuildProjectId = ANY(@ProjectIds)
            ORDER BY BuildProjectId, QueuedAt DESC, Id DESC
            """;

        var result = await db.QueryAsync<BuildRunDto>(
            sql,
            new { ProjectIds = projectIds.ToArray() },
            transaction: tx());
        return result.Select(static x => x.ToDomain()).ToDictionary(static x => x.BuildProjectId);
    }

    public async Task<IEnumerable<BuildRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM BuildRuns
            WHERE Status = @Status
            ORDER BY QueuedAt ASC, Id ASC
            LIMIT @Limit
            """;
        var result = await db.QueryAsync<BuildRunDto>(
            sql,
            new { Status = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued), Limit = limit },
            transaction: tx());
        return result.Select(static x => x.ToDomain());
    }

    public async Task<BuildRun?> TryClaimAsync(Guid id, DateTimeOffset startedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildRuns
            SET Status = @PreparingStatus,
                StartedAt = @StartedAt
            WHERE Id = @Id
              AND Status = @QueuedStatus
            RETURNING *
            """;
        var result = await db.QuerySingleOrDefaultAsync<BuildRunDto>(
            sql,
            new
            {
                Id = id,
                StartedAt = startedAt.UtcDateTime,
                QueuedStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued),
                PreparingStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Preparing)
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> HasActiveRunAsync(Guid projectId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1 FROM BuildRuns
                WHERE BuildProjectId = @ProjectId
                  AND Status = ANY(@Statuses))
            """;
        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                ProjectId = projectId,
                Statuses = new[]
                {
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Preparing),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Running)
                }
            },
            transaction: tx());
    }

    public async Task<BuildRun?> CancelQueuedOrRunningAsync(Guid id, DateTimeOffset cancelledAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildRuns
            SET Status = @CancelledStatus,
                CompletedAt = @CancelledAt,
                ErrorCode = @ErrorCode,
                ErrorMessage = @Reason
            WHERE Id = @Id
              AND Status = ANY(@ActiveStatuses)
            RETURNING *
            """;
        var result = await db.QuerySingleOrDefaultAsync<BuildRunDto>(
            sql,
            new
            {
                Id = id,
                CancelledAt = cancelledAt.UtcDateTime,
                ErrorCode = "build.cancelled",
                Reason = reason,
                CancelledStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Cancelled),
                ActiveStatuses = new[]
                {
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Preparing),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Running)
                }
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<BuildRun?> InterruptQueuedOrRunningAsync(Guid id, DateTimeOffset interruptedAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE BuildRuns
            SET Status = @InterruptedStatus,
                CompletedAt = @InterruptedAt,
                ErrorCode = @ErrorCode,
                ErrorMessage = @Reason
            WHERE Id = @Id
              AND Status = ANY(@ActiveStatuses)
            RETURNING *
            """;
        var result = await db.QuerySingleOrDefaultAsync<BuildRunDto>(
            sql,
            new
            {
                Id = id,
                InterruptedAt = interruptedAt.UtcDateTime,
                ErrorCode = "build.interrupted",
                Reason = reason,
                InterruptedStatus = EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Interrupted),
                ActiveStatuses = new[]
                {
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Queued),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Preparing),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Running)
                }
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IReadOnlyList<BuildRun>> DeleteTerminalRunsBeyondRetentionAsync(
        Guid projectId,
        int keepRunCount,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH RankedRuns AS (
                SELECT Id,
                       ROW_NUMBER() OVER (
                           ORDER BY COALESCE(CompletedAt, StartedAt, QueuedAt) DESC,
                                    QueuedAt DESC,
                                    Id DESC
                       ) AS RunRank
                FROM BuildRuns
                WHERE BuildProjectId = @ProjectId
                  AND Status = ANY(@TerminalStatuses)
            ),
            DeletedRuns AS (
                DELETE FROM BuildRuns runs
                USING RankedRuns ranked
                WHERE runs.Id = ranked.Id
                  AND ranked.RunRank > @KeepRunCount
                RETURNING runs.*
            )
            SELECT *
            FROM DeletedRuns
            ORDER BY COALESCE(CompletedAt, StartedAt, QueuedAt) ASC,
                     QueuedAt ASC,
                     Id ASC
            """;

        var result = await db.QueryAsync<BuildRunDto>(
            sql,
            new
            {
                ProjectId = projectId,
                KeepRunCount = Math.Max(1, keepRunCount),
                TerminalStatuses = new[]
                {
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Succeeded),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Failed),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.TimedOut),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Cancelled),
                    EnumFormatter<BuildRunStatus>.GetValue(BuildRunStatus.Interrupted)
                }
            },
            transaction: tx());
        return result.Select(static x => x.ToDomain()).ToArray();
    }

    private static BuildRunParameters ToParameters(BuildRun run)
        => new
        (
            run.Id,
            run.BuildProjectId,
            run.ProjectNameSnapshot,
            run.GitRepositoryId,
            run.GitRepositoryNameSnapshot,
            run.Branch,
            run.ResolvedCommitSha,
            run.ContextPath,
            run.DockerfilePath,
            run.Target,
            BuildMappers.SerializeBuildArgs(run.BuildArgsSnapshot),
            BuildMappers.SerializeStringList(run.BuildSecretIdsSnapshot),
            BuildMappers.SerializePlatformSnapshot(run.PlatformSnapshot),
            BuildMappers.SerializeRegistrySnapshot(run.RegistrySnapshot),
            run.ImageRepository,
            BuildMappers.SerializeStringList(run.TagTemplatesSnapshot),
            BuildMappers.SerializeStringList(run.ImageReferences),
            EnumFormatter<BuildRunTrigger>.GetValue(run.Trigger),
            run.TriggerSourceId,
            EnumFormatter<BuildRunStatus>.GetValue(run.Status),
            run.ImageDigest,
            run.TimeoutSeconds,
            run.QueuedAt.UtcDateTime,
            run.StartedAt?.UtcDateTime,
            run.CompletedAt?.UtcDateTime,
            run.ExitCode,
            run.ErrorCode,
            run.ErrorMessage,
            run.TriggeredByActorId
        );
}

internal sealed class BuildRunLogRepository(IDbConnection db, Func<IDbTransaction> tx) : IBuildRunLogRepository
{
    public Task<int> AddAsync(BuildRunLogEntry entry, CancellationToken cancellationToken)
        => AddRangeAsync(new[] { entry }, cancellationToken);

    public Task<int> AddRangeAsync(IReadOnlyCollection<BuildRunLogEntry> entries, CancellationToken cancellationToken)
    {
        if (entries.Count == 0)
            return Task.FromResult(0);

        var batch = entries.ToArray();
        const string sql = """
            INSERT INTO BuildRunLogs (Id, BuildRunId, CreatedAt, Stream, Message)
            SELECT Id, BuildRunId, CreatedAt, Stream, Message
            FROM unnest(
                @Ids::uuid[],
                @BuildRunIds::uuid[],
                @CreatedAts::timestamptz[],
                @Streams::text[],
                @Messages::text[])
                AS logs(Id, BuildRunId, CreatedAt, Stream, Message)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = batch.Select(static entry => entry.Id).ToArray(),
                BuildRunIds = batch.Select(static entry => entry.BuildRunId).ToArray(),
                CreatedAts = batch.Select(static entry => entry.CreatedAt.UtcDateTime).ToArray(),
                Streams = batch.Select(static entry => RemovePostgresNullBytes(entry.Stream)).ToArray(),
                Messages = batch.Select(static entry => RemovePostgresNullBytes(entry.Message)).ToArray()
            },
            transaction: tx());
    }

    private static string RemovePostgresNullBytes(string value)
        => value.Contains('\0', StringComparison.Ordinal)
            ? value.Replace("\0", string.Empty, StringComparison.Ordinal)
            : value;

    public async Task<IReadOnlyList<BuildRunLogEntry>> GetByRunAsync(Guid buildRunId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, BuildRunId, CreatedAt, Stream, Message
            FROM BuildRunLogs
            WHERE BuildRunId = @BuildRunId
            ORDER BY CreatedAt ASC, Id ASC
            """;
        var result = await db.QueryAsync<BuildRunLogDto>(sql, new { BuildRunId = buildRunId }, transaction: tx());
        return result.Select(static x => x.ToDomain()).ToArray();
    }
}
