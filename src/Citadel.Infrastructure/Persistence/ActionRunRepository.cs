using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using static Infrastructure.TypeHandlers.FormattingExtensions;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class ActionRunRepository(IDbConnection db, Func<IDbTransaction> tx) : IActionRunRepository
{
    public Task<int> AddAsync(ActionRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO ActionRuns (
                Id, ActionId, ActionName, Trigger, Status, RunAsActorId, TriggeredByActorId,
                ArgsJson, CodeSnapshot, CodeHash, TimeoutSeconds, QueuedAt, StartedAt, FinishedAt,
                DurationMs, ExitCode, Logs, ErrorMessage)
            VALUES (
                @Id, @ActionId, @ActionName, @Trigger, @Status, @RunAsActorId, @TriggeredByActorId,
                @ArgsJson::jsonb, @CodeSnapshot, @CodeHash, @TimeoutSeconds, @QueuedAt, @StartedAt, @FinishedAt,
                @DurationMs, @ExitCode, @Logs, @ErrorMessage)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                run.ActionId,
                run.ActionName,
                Trigger = EnumFormatter<ActionRunTrigger>.GetValue(run.Trigger),
                Status = EnumFormatter<ActionRunStatus>.GetValue(run.Status),
                run.RunAsActorId,
                run.TriggeredByActorId,
                run.ArgsJson,
                run.CodeSnapshot,
                run.CodeHash,
                run.TimeoutSeconds,
                run.QueuedAt,
                run.StartedAt,
                run.FinishedAt,
                run.DurationMs,
                run.ExitCode,
                run.Logs,
                run.ErrorMessage
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(ActionRun run, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE ActionRuns
            SET Status = @Status,
                StartedAt = @StartedAt,
                FinishedAt = @FinishedAt,
                DurationMs = @DurationMs,
                ExitCode = @ExitCode,
                Logs = @Logs,
                ErrorMessage = @ErrorMessage
            WHERE Id = @Id
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                run.Id,
                Status = EnumFormatter<ActionRunStatus>.GetValue(run.Status),
                run.StartedAt,
                run.FinishedAt,
                run.DurationMs,
                run.ExitCode,
                run.Logs,
                run.ErrorMessage
            },
            transaction: tx());
    }

    public async Task<ActionRun?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM ActionRuns WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<ActionRunDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<ActionRun>> GetByActionAsync(Guid actionId, int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM ActionRuns
            WHERE ActionId = @ActionId
            ORDER BY QueuedAt DESC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<ActionRunDto>(sql, new { ActionId = actionId, Limit = limit }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<ActionRun>> GetQueuedAsync(int limit, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM ActionRuns
            WHERE Status = @Status
            ORDER BY QueuedAt ASC
            LIMIT @Limit
            """;

        var result = await db.QueryAsync<ActionRunDto>(
            sql,
            new { Status = EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Queued), Limit = limit },
            transaction: tx());
        return result.ToDomain();
    }

    public async Task<ActionRun?> GetLatestByActionAsync(Guid actionId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM ActionRuns
            WHERE ActionId = @ActionId
            ORDER BY QueuedAt DESC
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<ActionRunDto>(
            sql,
            new { ActionId = actionId },
            transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> HasActiveRunAsync(Guid actionId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT EXISTS (
                SELECT 1
                FROM ActionRuns
                WHERE ActionId = @ActionId
                  AND Status = ANY(@Statuses)
            )
            """;

        return db.ExecuteScalarAsync<bool>(
            sql,
            new
            {
                ActionId = actionId,
                Statuses = new[]
                {
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Queued),
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Running)
                }
            },
            transaction: tx());
    }

    public async Task<bool> TryMarkRunningAsync(Guid id, DateTime startedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE ActionRuns
            SET Status = @RunningStatus,
                StartedAt = @StartedAt
            WHERE Id = @Id
              AND Status = @QueuedStatus
            """;

        var rows = await db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                StartedAt = startedAt,
                QueuedStatus = EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Queued),
                RunningStatus = EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Running)
            },
            transaction: tx());
        return rows > 0;
    }

    public Task<int> CancelQueuedOrRunningAsync(Guid id, DateTime cancelledAt, string reason, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE ActionRuns
            SET Status = @CancelledStatus,
                FinishedAt = @CancelledAt,
                DurationMs = CASE
                    WHEN StartedAt IS NULL THEN NULL
                    ELSE GREATEST(0, (EXTRACT(EPOCH FROM (@CancelledAt - StartedAt)) * 1000)::bigint)
                END,
                ErrorMessage = @Reason
            WHERE Id = @Id
              AND Status = ANY(@ActiveStatuses)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                CancelledAt = cancelledAt,
                Reason = reason,
                CancelledStatus = EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Cancelled),
                ActiveStatuses = new[]
                {
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Queued),
                    EnumFormatter<ActionRunStatus>.GetValue(ActionRunStatus.Running)
                }
            },
            transaction: tx());
    }
}
