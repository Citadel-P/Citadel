using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence;

internal sealed class UserPreferencesRepository(IDbConnection db, Func<IDbTransaction> tx) : IUserPreferencesRepository
{
    public async Task<UserPreferences?> GetAsync(Guid userId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT UserId, TimeZone, DateTimeFormat, Theme, UpdatedAt
            FROM UserPreferences
            WHERE UserId = @UserId
            LIMIT 1
            """;

        var row = await db.QuerySingleOrDefaultAsync<UserPreferencesDto>(
            sql,
            new { UserId = userId },
            transaction: tx());

        return row is null
            ? null
            : UserPreferences.Create(
                row.UserId,
                row.TimeZone,
                Enum.Parse<UserDateTimeFormat>(row.DateTimeFormat),
                Enum.Parse<UserTheme>(row.Theme),
                row.UpdatedAt);
    }

    public Task<int> UpsertAsync(UserPreferences preferences, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO UserPreferences (
                UserId,
                TimeZone,
                DateTimeFormat,
                Theme,
                UpdatedAt)
            VALUES (
                @UserId,
                @TimeZone,
                @DateTimeFormat,
                @Theme,
                @UpdatedAt)
            ON CONFLICT (UserId) DO UPDATE
            SET TimeZone = EXCLUDED.TimeZone,
                DateTimeFormat = EXCLUDED.DateTimeFormat,
                Theme = EXCLUDED.Theme,
                UpdatedAt = EXCLUDED.UpdatedAt
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                preferences.UserId,
                preferences.TimeZone,
                DateTimeFormat = preferences.DateTimeFormat.ToString(),
                Theme = preferences.Theme.ToString(),
                preferences.UpdatedAt,
            },
            transaction: tx());
    }
}
