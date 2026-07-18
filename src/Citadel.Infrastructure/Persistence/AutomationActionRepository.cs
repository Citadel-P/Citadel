using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using static Infrastructure.TypeHandlers.FormattingExtensions;
using System.Data;
using System.Text.Json;

namespace Infrastructure.Persistence;

internal sealed class AutomationActionRepository(IDbConnection db, Func<IDbTransaction> tx) : IAutomationActionRepository
{
    public async Task<int> AddAsync(
        AutomationAction action,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null,
        Guid? tagCreatedByActorId = null)
    {
        action.Validate();

        string sql = ResourceTagSql.InputTagsCte + """
            inserted_action AS (
                INSERT INTO Actions (
                Id, Name, Description, Code, DefaultArgsJson, Enabled,
                ScheduleEnabled, ScheduleCron, ScheduleTimeZone,
                Webhook, TimeoutSeconds, AlertOnFailure,
                RunAsActorId, LastScheduledRunAt, ControlState, CurrentRunId, ControlStartedAt, RowVersion,
                CreatedByActorId, CreatedAt, UpdatedAt)
                SELECT
                @Id, @Name, @Description, @Code, @DefaultArgsJson::jsonb, @Enabled,
                @ScheduleEnabled, @ScheduleCron, @ScheduleTimeZone,
                @Webhook::jsonb, @TimeoutSeconds, @AlertOnFailure,
                @RunAsActorId, @LastScheduledRunAt, @ControlState, @CurrentRunId, @ControlStartedAt, @RowVersion,
                @CreatedByActorId, @CreatedAt, @UpdatedAt
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_action", "a") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_action", "inserted_action", "inserted_tags");

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(
            sql,
            new
            {
                action.Id,
                action.Name,
                action.Description,
                action.Code,
                action.DefaultArgsJson,
                action.Enabled,
                action.ScheduleEnabled,
                action.ScheduleCron,
                action.ScheduleTimeZone,
                Webhook = action.Webhook is null
                    ? null
                    : JsonSerializer.Serialize(action.Webhook, AutomationJsonContext.Default.AutomationWebhookConfig),
                action.TimeoutSeconds,
                action.AlertOnFailure,
                action.RunAsActorId,
                action.LastScheduledRunAt,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(action.ControlState),
                action.CurrentRunId,
                action.ControlStartedAt,
                action.RowVersion,
                action.CreatedByActorId,
                action.CreatedAt,
                action.UpdatedAt,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.AutomationAction),
                TagIds = tagIdArray,
                TagCreatedByActorId = tagCreatedByActorId ?? action.CreatedByActorId
            },
            transaction: tx());

        action.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public Task<int> UpdateAsync(AutomationAction action, CancellationToken cancellationToken)
    {
        action.Validate();

        const string sql = """
            UPDATE Actions
            SET Name = @Name,
                Description = @Description,
                Code = @Code,
                DefaultArgsJson = @DefaultArgsJson::jsonb,
                Enabled = @Enabled,
                ScheduleEnabled = @ScheduleEnabled,
                ScheduleCron = @ScheduleCron,
                ScheduleTimeZone = @ScheduleTimeZone,
                Webhook = @Webhook::jsonb,
                TimeoutSeconds = @TimeoutSeconds,
                AlertOnFailure = @AlertOnFailure,
                RunAsActorId = @RunAsActorId,
                LastScheduledRunAt = @LastScheduledRunAt,
                ControlState = @ControlState,
                CurrentRunId = @CurrentRunId,
                ControlStartedAt = @ControlStartedAt,
                RowVersion = @RowVersion,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                action.Id,
                action.Name,
                action.Description,
                action.Code,
                action.DefaultArgsJson,
                action.Enabled,
                action.ScheduleEnabled,
                action.ScheduleCron,
                action.ScheduleTimeZone,
                Webhook = action.Webhook is null
                    ? null
                    : JsonSerializer.Serialize(action.Webhook, AutomationJsonContext.Default.AutomationWebhookConfig),
                action.TimeoutSeconds,
                action.AlertOnFailure,
                action.RunAsActorId,
                action.LastScheduledRunAt,
                ControlState = EnumFormatter<ResourceControlState>.GetValue(action.ControlState),
                action.CurrentRunId,
                action.ControlStartedAt,
                action.RowVersion,
                action.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Actions WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id }, transaction: tx());
    }

    public async Task<AutomationAction?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT a.*,
                {{ResourceTagSql.TagAggregate("a")}}
            FROM Actions a
            WHERE a.Id = @Id
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<AutomationActionDto>(
            sql,
            new
            {
                Id = id,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.AutomationAction)
            },
            transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<AutomationAction>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = $$"""
            SELECT a.*,
                {{ResourceTagSql.TagAggregate("a")}}
            FROM Actions a
            WHERE {{ResourceTagSql.FilterPredicate("a")}}
            ORDER BY Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<AutomationActionDto>(
            sql,
            new
            {
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.AutomationAction),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());

        return result.ToDomain();
    }

    public async Task<IEnumerable<AutomationAction>> GetScheduledAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM Actions
            WHERE Enabled AND ScheduleEnabled AND ScheduleCron IS NOT NULL
            ORDER BY Name ASC
            """;

        var result = await db.QueryAsync<AutomationActionDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<AutomationAction>> GetAuthorizedAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken,
        IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT a.*,
                {{ResourceTagSql.TagAggregate("a")}}
            FROM Actions a
            WHERE {{AuthorizationSql.ResourcePredicatePrefix}}a.Id{{AuthorizationSql.ResourcePredicateSuffix}}
              AND {{ResourceTagSql.FilterPredicate("a")}}
            ORDER BY a.Name ASC
            """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<AutomationActionDto>(
            sql,
            new
            {
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.AutomationAction),
                TagIds = tagIdArray,
                TagIdsLength = tagIdArray.Length
            },
            transaction: tx());

        return result.ToDomain();
    }

    public Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Actions WHERE lower(Name) = lower(@Name))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name }, transaction: tx());
    }

    public Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Actions WHERE lower(Name) = lower(@Name) AND Id <> @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id }, transaction: tx());
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
                FROM Actions a
                WHERE a.Id = @Id
                  AND {{AuthorizationSql.ResourcePredicatePrefix}}a.Id{{AuthorizationSql.ResourcePredicateSuffix}}
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

    public async Task<bool> TryMarkScheduledAsync(Guid id, DateTime scheduledMinuteUtc, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Actions
            SET LastScheduledRunAt = @ScheduledMinuteUtc,
                UpdatedAt = @UpdatedAt,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND (LastScheduledRunAt IS NULL OR LastScheduledRunAt < @ScheduledMinuteUtc)
            """;

        var rows = await db.ExecuteAsync(
            sql,
            new { Id = id, ScheduledMinuteUtc = scheduledMinuteUtc, UpdatedAt = DateTime.UtcNow },
            transaction: tx());
        return rows > 0;
    }

    public Task<int> MarkProcessingAsync(Guid id, Guid runId, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Actions
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
            UPDATE Actions
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

    public async Task<IEnumerable<AutomationAction>> GetStuckActionsAsync(CancellationToken cancellationToken = default)
    {
        string sql = $$"""
            SELECT a.*,
                {{ResourceTagSql.TagAggregate("a")}}
            FROM Actions a
            LEFT JOIN ActionRuns r ON r.Id = a.CurrentRunId
            WHERE a.ControlState = @ControlState
              AND (
                  a.CurrentRunId IS NULL
                  OR r.Id IS NULL
                  OR r.Status <> ALL(@ActiveStatuses)
              )
            ORDER BY a.ControlStartedAt ASC
            """;

        var result = await db.QueryAsync<AutomationActionDto>(
            sql,
            new
            {
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ActiveStatuses = new[]
                {
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Queued),
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Running)
                },
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.AutomationAction)
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
            UPDATE Actions
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
}
