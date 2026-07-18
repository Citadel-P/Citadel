using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class UserMfaRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserMfaRepository
{
    public async Task<UserMfaSettings?> GetSettingsAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT UserId, ProtectedTotpSecret, LastAcceptedTimeStep, EnabledAt, CreatedAt
            FROM UserMfaSettings
            WHERE UserId = @UserId
            LIMIT 1
            """;

        return await db.QuerySingleOrDefaultAsync<UserMfaSettings>(
            sql,
            new { UserId = userId },
            transaction: tx());
    }

    public async Task<int> CountUnusedRecoveryCodesAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT COUNT(*)::int
            FROM UserMfaRecoveryCodes
            WHERE UserId = @UserId
              AND UsedAt IS NULL
            """;

        return await db.QuerySingleAsync<int>(
            sql,
            new { UserId = userId },
            transaction: tx());
    }

    public Task<int> UpsertSettingsAsync(UserMfaSettings settings, CancellationToken cancellationToken)
    {
        settings.Validate();

        const string sql = """
            INSERT INTO UserMfaSettings (UserId, ProtectedTotpSecret, LastAcceptedTimeStep, EnabledAt, CreatedAt)
            VALUES (@UserId, @ProtectedTotpSecret, @LastAcceptedTimeStep, @EnabledAt, @CreatedAt)
            ON CONFLICT (UserId) DO UPDATE SET
                ProtectedTotpSecret = EXCLUDED.ProtectedTotpSecret,
                LastAcceptedTimeStep = EXCLUDED.LastAcceptedTimeStep,
                EnabledAt = EXCLUDED.EnabledAt
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                settings.UserId,
                settings.ProtectedTotpSecret,
                settings.LastAcceptedTimeStep,
                settings.EnabledAt,
                settings.CreatedAt
            },
            transaction: tx());
    }

    public Task<int> TryAcceptTimeStepAsync(Guid userId, long matchedTimeStep, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE UserMfaSettings
            SET LastAcceptedTimeStep = @MatchedTimeStep
            WHERE UserId = @UserId
              AND (
                  LastAcceptedTimeStep IS NULL
                  OR LastAcceptedTimeStep < @MatchedTimeStep
              )
            """;

        return db.ExecuteAsync(
            sql,
            new { UserId = userId, MatchedTimeStep = matchedTimeStep },
            transaction: tx());
    }

    public Task<int> DeleteSettingsAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM UserMfaSettings WHERE UserId = @UserId";
        return db.ExecuteAsync(sql, new { UserId = userId }, transaction: tx());
    }

    public async Task<int> ReplaceRecoveryCodesAsync(Guid userId, IReadOnlyCollection<UserMfaRecoveryCode> codes, CancellationToken cancellationToken)
    {
        await DeleteRecoveryCodesAsync(userId, cancellationToken);
        if (codes.Count == 0)
            return 0;

        foreach (var code in codes)
            code.Validate();

        const string sql = """
            INSERT INTO UserMfaRecoveryCodes (Id, UserId, CodeHash, UsedAt, CreatedAt)
            VALUES (@Id, @UserId, @CodeHash, @UsedAt, @CreatedAt)
            """;

        return await db.ExecuteAsync(
            sql,
            codes.Select(code => new
            {
                code.Id,
                code.UserId,
                code.CodeHash,
                code.UsedAt,
                code.CreatedAt
            }),
            transaction: tx());
    }

    public Task<int> DeleteRecoveryCodesAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM UserMfaRecoveryCodes WHERE UserId = @UserId";
        return db.ExecuteAsync(sql, new { UserId = userId }, transaction: tx());
    }

    public Task<int> TryUseRecoveryCodeAsync(Guid userId, string codeHash, DateTime usedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE UserMfaRecoveryCodes
            SET UsedAt = @UsedAt
            WHERE UserId = @UserId
              AND CodeHash = @CodeHash
              AND UsedAt IS NULL
            """;

        return db.ExecuteAsync(
            sql,
            new { UserId = userId, CodeHash = codeHash, UsedAt = usedAt },
            transaction: tx());
    }

    public Task<int> AddSetupSessionAsync(MfaSetupSession session, CancellationToken cancellationToken)
    {
        session.Validate();

        const string sql = """
            INSERT INTO MfaSetupSessions (Id, UserId, ProtectedTotpSecret, ExpiresAt, ConsumedAt, CreatedAt)
            VALUES (@Id, @UserId, @ProtectedTotpSecret, @ExpiresAt, @ConsumedAt, @CreatedAt)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                session.Id,
                session.UserId,
                session.ProtectedTotpSecret,
                session.ExpiresAt,
                session.ConsumedAt,
                session.CreatedAt
            },
            transaction: tx());
    }

    public async Task<MfaSetupSession?> GetSetupSessionAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, UserId, ProtectedTotpSecret, ExpiresAt, ConsumedAt, CreatedAt
            FROM MfaSetupSessions
            WHERE Id = @Id
            LIMIT 1
            """;

        return await db.QuerySingleOrDefaultAsync<MfaSetupSession>(
            sql,
            new { Id = id },
            transaction: tx());
    }

    public async Task<MfaSetupSession?> GetActiveSetupSessionByUserAsync(Guid userId, DateTime now, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, UserId, ProtectedTotpSecret, ExpiresAt, ConsumedAt, CreatedAt
            FROM MfaSetupSessions
            WHERE UserId = @UserId
              AND ConsumedAt IS NULL
              AND ExpiresAt > @Now
            ORDER BY CreatedAt DESC
            LIMIT 1
            """;

        return await db.QuerySingleOrDefaultAsync<MfaSetupSession>(
            sql,
            new { UserId = userId, Now = now },
            transaction: tx());
    }

    public Task<int> TryConsumeSetupSessionAsync(Guid id, DateTime consumedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE MfaSetupSessions
            SET ConsumedAt = @ConsumedAt
            WHERE Id = @Id
              AND ConsumedAt IS NULL
              AND ExpiresAt > @ConsumedAt
            """;

        return db.ExecuteAsync(
            sql,
            new { Id = id, ConsumedAt = consumedAt },
            transaction: tx());
    }

    public Task<int> DeleteSetupSessionsAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM MfaSetupSessions WHERE UserId = @UserId";
        return db.ExecuteAsync(sql, new { UserId = userId }, transaction: tx());
    }

    public Task<int> DeleteExpiredSetupSessionsAsync(DateTime now, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM MfaSetupSessions WHERE ExpiresAt <= @Now";
        return db.ExecuteAsync(sql, new { Now = now }, transaction: tx());
    }
}

internal sealed class MfaChallengeRepository(IDbConnection db, Func<IDbTransaction> tx) : IMfaChallengeRepository
{
    public Task<int> AddAsync(MfaChallenge challenge, CancellationToken cancellationToken)
    {
        challenge.Validate();

        const string sql = """
            INSERT INTO MfaChallenges (Id, UserId, ExpiresAt, FailedAttempts, ConsumedAt, CreatedAt)
            VALUES (@Id, @UserId, @ExpiresAt, @FailedAttempts, @ConsumedAt, @CreatedAt)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                challenge.Id,
                challenge.UserId,
                challenge.ExpiresAt,
                challenge.FailedAttempts,
                challenge.ConsumedAt,
                challenge.CreatedAt
            },
            transaction: tx());
    }

    public async Task<MfaChallenge?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, UserId, ExpiresAt, FailedAttempts, ConsumedAt, CreatedAt
            FROM MfaChallenges
            WHERE Id = @Id
            LIMIT 1
            """;

        return await db.QuerySingleOrDefaultAsync<MfaChallenge>(
            sql,
            new { Id = id },
            transaction: tx());
    }

    public Task<int> TryIncrementFailedAttemptsAsync(Guid id, DateTime now, int maxFailedAttempts, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE MfaChallenges
            SET FailedAttempts = FailedAttempts + 1
            WHERE Id = @Id
              AND ConsumedAt IS NULL
              AND ExpiresAt > @Now
              AND FailedAttempts < @MaxFailedAttempts
            """;

        return db.ExecuteAsync(
            sql,
            new { Id = id, Now = now, MaxFailedAttempts = maxFailedAttempts },
            transaction: tx());
    }

    public Task<int> TryConsumeAsync(Guid id, DateTime consumedAt, int maxFailedAttempts, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE MfaChallenges
            SET ConsumedAt = @ConsumedAt
            WHERE Id = @Id
              AND ConsumedAt IS NULL
              AND ExpiresAt > @ConsumedAt
              AND FailedAttempts < @MaxFailedAttempts
            """;

        return db.ExecuteAsync(
            sql,
            new { Id = id, ConsumedAt = consumedAt, MaxFailedAttempts = maxFailedAttempts },
            transaction: tx());
    }

    public Task<int> DeleteForUserAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM MfaChallenges WHERE UserId = @UserId";
        return db.ExecuteAsync(sql, new { UserId = userId }, transaction: tx());
    }

    public Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM MfaChallenges WHERE ExpiresAt <= @Now";
        return db.ExecuteAsync(sql, new { Now = now }, transaction: tx());
    }
}
