using Microsoft.Data.Sqlite;
using Polly;
using Polly.Retry;

namespace Infrastructure.Persistence;

/// <summary>
/// Provides predefined retry policies for handling transient database errors.
/// </summary>
internal static class DbRetryPolicies
{
    public static bool IsBusy(SqliteException ex) =>
        ex.SqliteErrorCode is
            5   // SQLITE_BUSY
            or 6   // SQLITE_LOCKED
            or 261 // SQLITE_BUSY_RECOVERY
            or 517; // SQLITE_BUSY_SNAPSHOT

    public static readonly AsyncRetryPolicy RetryOnBusy =
        Policy
            .Handle<SqliteException>(IsBusy)
            .WaitAndRetryAsync(
                retryCount: 5,
                sleepDurationProvider: attempt =>
                {
                    // Exponential backoff + jitter
                    var baseDelay = TimeSpan.FromMilliseconds(100 * Math.Pow(2, attempt - 1));
                    var jitterMs = Random.Shared.Next(0, 100);
                    return baseDelay + TimeSpan.FromMilliseconds(jitterMs);
                });
}