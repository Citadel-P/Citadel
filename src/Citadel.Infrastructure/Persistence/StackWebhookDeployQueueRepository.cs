using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class StackWebhookDeployQueueRepository(
    IDbConnection db,
    Func<IDbTransaction> tx) : IStackWebhookDeployQueueRepository
{
    public Task<int> AddAsync(
        StackWebhookDeployQueueItem item,
        CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO StackWebhookDeployQueue
                (Id, StackId, GitRepositoryId, ExpectedStackReleaseId, Branch,
                 ExpectedSpecFingerprint, DispatchedCommitSha, Status, Attempts,
                 QueuedAt, AvailableAt, StartedAt, LastError)
            VALUES
                (@Id, @StackId, @GitRepositoryId, @ExpectedStackReleaseId, @Branch,
                 @ExpectedSpecFingerprint, @DispatchedCommitSha, @Status, @Attempts,
                 @QueuedAt, @AvailableAt, @StartedAt, @LastError)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                item.Id,
                item.StackId,
                item.GitRepositoryId,
                item.ExpectedStackReleaseId,
                item.Branch,
                item.ExpectedSpecFingerprint,
                item.DispatchedCommitSha,
                Status = item.Status.ToString(),
                item.Attempts,
                item.QueuedAt,
                item.AvailableAt,
                item.StartedAt,
                item.LastError
            },
            transaction: tx());
    }

    public async Task<IReadOnlyList<Guid>> GetReadyIdsAsync(
        int limit,
        DateTime availableAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id
            FROM StackWebhookDeployQueue
            WHERE Status = @QueuedStatus
              AND AvailableAt <= @AvailableAt
            ORDER BY AvailableAt, QueuedAt, Id
            LIMIT @Limit
            """;

        var ids = await db.QueryAsync<Guid>(
            sql,
            new
            {
                QueuedStatus = nameof(StackWebhookDeployQueueStatus.Queued),
                AvailableAt = availableAt,
                Limit = Math.Max(1, limit)
            },
            transaction: tx());
        return ids.AsList();
    }

    public async Task<StackWebhookDeployQueueItem?> TryClaimAsync(
        Guid id,
        DateTime startedAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE StackWebhookDeployQueue
            SET Status = @ProcessingStatus,
                Attempts = Attempts + 1,
                StartedAt = @StartedAt,
                LastError = NULL
            WHERE Id = @Id
              AND Status = @QueuedStatus
              AND AvailableAt <= @StartedAt
            RETURNING *
            """;

        var item = await db.QuerySingleOrDefaultAsync<StackWebhookDeployQueueItemDto>(
            sql,
            new
            {
                Id = id,
                StartedAt = startedAt,
                QueuedStatus = nameof(StackWebhookDeployQueueStatus.Queued),
                ProcessingStatus = nameof(StackWebhookDeployQueueStatus.Processing)
            },
            transaction: tx());
        return item?.ToDomain();
    }

    public Task<int> RetryAsync(
        Guid id,
        string error,
        DateTime availableAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE StackWebhookDeployQueue
            SET Status = @QueuedStatus,
                AvailableAt = @AvailableAt,
                StartedAt = NULL,
                LastError = @LastError
            WHERE Id = @Id
              AND Status = @ProcessingStatus
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                AvailableAt = availableAt,
                LastError = Truncate(error, 2000),
                QueuedStatus = nameof(StackWebhookDeployQueueStatus.Queued),
                ProcessingStatus = nameof(StackWebhookDeployQueueStatus.Processing)
            },
            transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM StackWebhookDeployQueue WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id }, transaction: tx());
    }

    public Task<int> RequeueInterruptedAsync(
        DateTime availableAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE StackWebhookDeployQueue
            SET Status = @QueuedStatus,
                AvailableAt = @AvailableAt,
                StartedAt = NULL,
                LastError = COALESCE(LastError, 'Interrupted by application shutdown.')
            WHERE Status = @ProcessingStatus
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                AvailableAt = availableAt,
                QueuedStatus = nameof(StackWebhookDeployQueueStatus.Queued),
                ProcessingStatus = nameof(StackWebhookDeployQueueStatus.Processing)
            },
            transaction: tx());
    }

    private static string Truncate(string value, int maxLength)
        => value.Length <= maxLength ? value : value[..maxLength];
}

internal sealed record StackWebhookDeployQueueItemDto(
    Guid Id,
    Guid StackId,
    Guid GitRepositoryId,
    Guid ExpectedStackReleaseId,
    string Branch,
    string ExpectedSpecFingerprint,
    string? DispatchedCommitSha,
    string Status,
    int Attempts,
    DateTime QueuedAt,
    DateTime AvailableAt,
    DateTime? StartedAt,
    string? LastError)
{
    public StackWebhookDeployQueueItemDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            0,
            DateTime.MinValue,
            DateTime.MinValue,
            null,
            null)
    {
    }

    internal StackWebhookDeployQueueItem ToDomain()
        => new(
            Id,
            StackId,
            GitRepositoryId,
            ExpectedStackReleaseId,
            Branch,
            ExpectedSpecFingerprint,
            DispatchedCommitSha,
            Enum.Parse<StackWebhookDeployQueueStatus>(Status),
            Attempts,
            DateTime.SpecifyKind(QueuedAt, DateTimeKind.Utc),
            DateTime.SpecifyKind(AvailableAt, DateTimeKind.Utc),
            StartedAt.HasValue
                ? DateTime.SpecifyKind(StartedAt.Value, DateTimeKind.Utc)
                : null,
            LastError);
}
