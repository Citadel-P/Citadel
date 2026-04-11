using Npgsql;
using Polly;
using Polly.Retry;

namespace Infrastructure.Persistence;

/// </summary>
internal static class DbRetryPolicies
{
    // PostgreSQL Error Codes: https://www.postgresql.org/docs/current/errcodes-appendix.html
    private static readonly HashSet<string> TransientSqlStates =
    [
        "40001", // serialization_failure (Retryable)
        "40P01", // deadlock_detected (Retryable)
        "57P01", // admin_shutdown
        "57P02", // crash_shutdown
        "57P03", // cannot_connect_now
        "08001", // sqlclient_unable_to_establish_sql_connection
        "08006"  // connection_failure
    ];

    public static bool IsTransient(Exception ex) =>
        ex is NpgsqlException npgsqlEx && (npgsqlEx.IsTransient || TransientSqlStates.Contains(npgsqlEx.SqlState ?? ""));

    public static readonly AsyncRetryPolicy RetryOnTransient =
        Policy
            .Handle<NpgsqlException>(ex => IsTransient(ex))
            // Also handle SocketException or HttpRequestException if applicable
            .WaitAndRetryAsync(
                retryCount: 3,
                attempt => TimeSpan.FromMilliseconds(Math.Pow(2, attempt) * 100)
                           + TimeSpan.FromMilliseconds(Random.Shared.Next(0, 100)));
}