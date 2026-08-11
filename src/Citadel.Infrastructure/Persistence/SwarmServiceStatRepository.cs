using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Npgsql;
using NpgsqlTypes;

namespace Infrastructure.Persistence;

internal sealed class SwarmServiceStatRepository(IDbConnection db, Func<IDbTransaction> tx)
    : ISwarmServiceStatRepository
{
    public async Task<IReadOnlyList<SwarmServiceStatAttribution>> GetAttributionsAsync(
        Guid[] containerIds,
        CancellationToken cancellationToken)
    {
        if (containerIds.Length == 0)
            return [];

        const string sql = """
            SELECT container.Id AS ContainerId,
                   container.PlatformId,
                   task.DockerServiceId,
                   service.SwarmServiceId,
                   service.StackId,
                   service.Name AS ServiceName,
                   CASE
                       WHEN task.Slot IS NOT NULL THEN 'slot:' || task.Slot::text
                       ELSE 'node:' || task.DockerNodeId
                   END AS TaskKey,
                   task.DockerTaskId
            FROM Containers container
            INNER JOIN SwarmTaskProjections task
                ON task.PlatformId = container.PlatformId
               AND task.DockerNodeId = container.DockerNodeId
               AND task.DockerContainerId = container.DockerContainerId
            INNER JOIN SwarmServiceProjections service
                ON service.PlatformId = task.PlatformId
               AND service.DockerServiceId = task.DockerServiceId
            WHERE container.Id = ANY(@ContainerIds)
              AND container.IsSwarmTask
              AND service.Ownership <> 'System'
            """;

        return (await db.QueryAsync<SwarmServiceStatAttribution>(
            sql,
            new { ContainerIds = containerIds },
            transaction: tx())).AsList();
    }

    public async Task<IReadOnlyList<SwarmServiceStatSample>> GetStatsAggregatedAsync(
        SwarmServiceStatIdentity identity,
        int hours,
        CancellationToken cancellationToken)
    {
        var normalizedHours = Math.Clamp(hours, 1, 72);
        var parameters = new
        {
            identity.PlatformId,
            identity.DockerServiceId,
            identity.SwarmServiceId,
            identity.StackId,
            identity.ServiceName,
            Since = DateTimeOffset.UtcNow.AddHours(-normalizedHours).ToUnixTimeSeconds(),
            BucketSeconds = normalizedHours > 24 ? 300 : 60
        };

        if (identity.SwarmServiceId is not null)
            return (await db.QueryAsync<SwarmServiceStatSample>(
                ManagedServiceStatsSql,
                parameters,
                transaction: tx())).AsList();

        if (identity.StackId is not null)
            return (await db.QueryAsync<SwarmServiceStatSample>(
                StackServiceStatsSql,
                parameters,
                transaction: tx())).AsList();

        return (await db.QueryAsync<SwarmServiceStatSample>(
            DockerServiceStatsSql,
            parameters,
            transaction: tx())).AsList();
    }

    public async Task<int> BulkInsertAsync(
        IEnumerable<SwarmServiceStat> stats,
        CancellationToken cancellationToken)
    {
        _ = tx();
        if (db is not NpgsqlConnection connection)
            throw new InvalidOperationException("Swarm Service statistics require a PostgreSQL connection.");

        await using var importer = await connection.BeginBinaryImportAsync(
            """
            COPY SwarmServiceStats
                (Id, PlatformId, DockerServiceId, SwarmServiceId, StackId, ServiceName, TaskKey,
                 DockerTaskId, Created, MemoryActive, MemoryCache, CpuUsage, MemoryLimit, RxBytes, TxBytes)
            FROM STDIN (FORMAT BINARY)
            """,
            cancellationToken);

        var count = 0;
        foreach (var stat in stats)
        {
            await importer.StartRowAsync(cancellationToken);
            await importer.WriteAsync(stat.Id, NpgsqlDbType.Uuid, cancellationToken);
            await importer.WriteAsync(stat.PlatformId, NpgsqlDbType.Uuid, cancellationToken);
            await importer.WriteAsync(stat.DockerServiceId, NpgsqlDbType.Text, cancellationToken);
            await WriteNullableAsync(importer, stat.SwarmServiceId, NpgsqlDbType.Uuid, cancellationToken);
            await WriteNullableAsync(importer, stat.StackId, NpgsqlDbType.Uuid, cancellationToken);
            await importer.WriteAsync(stat.ServiceName, NpgsqlDbType.Text, cancellationToken);
            await importer.WriteAsync(stat.TaskKey, NpgsqlDbType.Text, cancellationToken);
            await importer.WriteAsync(stat.DockerTaskId, NpgsqlDbType.Text, cancellationToken);
            await importer.WriteAsync(stat.Created, NpgsqlDbType.Bigint, cancellationToken);
            await WriteNullableAsync(importer, stat.MemoryActive, NpgsqlDbType.Double, cancellationToken);
            await WriteNullableAsync(importer, stat.MemoryCache, NpgsqlDbType.Double, cancellationToken);
            await WriteNullableAsync(importer, stat.CpuUsage, NpgsqlDbType.Double, cancellationToken);
            await WriteNullableAsync(importer, stat.MemoryLimit, NpgsqlDbType.Double, cancellationToken);
            await WriteNullableAsync(importer, stat.RxBytes, NpgsqlDbType.Double, cancellationToken);
            await WriteNullableAsync(importer, stat.TxBytes, NpgsqlDbType.Double, cancellationToken);
            count++;
        }

        await importer.CompleteAsync(cancellationToken);
        return count;
    }

    public Task<int> RemoveOlderThanAsync(
        long createdBeforeEpochSeconds,
        CancellationToken cancellationToken)
    {
        const string sql = """
            WITH expired AS (
                SELECT ctid
                FROM SwarmServiceStats
                WHERE Created < @CreatedBefore
                ORDER BY Created
                LIMIT 5000
            )
            DELETE FROM SwarmServiceStats stats
            USING expired
            WHERE stats.ctid = expired.ctid
            """;

        return db.ExecuteAsync(
            sql,
            new { CreatedBefore = createdBeforeEpochSeconds },
            transaction: tx());
    }

    private const string ManagedServiceStatsSql = """
        WITH TaskBuckets AS (
            SELECT TaskKey,
                   (Created / @BucketSeconds) AS Bucket,
                   AVG(MemoryActive) AS MemoryActive,
                   AVG(MemoryCache) AS MemoryCache,
                   AVG(CpuUsage) AS CpuUsage,
                   AVG(MemoryLimit) AS MemoryLimit,
                   AVG(RxBytes) AS RxBytes,
                   AVG(TxBytes) AS TxBytes,
                   MIN(Created) AS Created
            FROM SwarmServiceStats
            WHERE (SwarmServiceId = @SwarmServiceId
                   OR (SwarmServiceId IS NULL
                       AND StackId IS NULL
                       AND PlatformId = @PlatformId
                       AND DockerServiceId = @DockerServiceId))
              AND Created > @Since
            GROUP BY TaskKey, (Created / @BucketSeconds)
        )
        SELECT SUM(MemoryActive) AS MemoryActive,
               SUM(MemoryCache) AS MemoryCache,
               SUM(CpuUsage) AS CpuUsage,
               SUM(MemoryLimit) AS MemoryLimit,
               SUM(RxBytes) AS RxBytes,
               SUM(TxBytes) AS TxBytes,
               MIN(Created) AS Created
        FROM TaskBuckets
        GROUP BY Bucket
        ORDER BY Bucket
        """;

    private const string StackServiceStatsSql = """
        WITH TaskBuckets AS (
            SELECT TaskKey,
                   (Created / @BucketSeconds) AS Bucket,
                   AVG(MemoryActive) AS MemoryActive,
                   AVG(MemoryCache) AS MemoryCache,
                   AVG(CpuUsage) AS CpuUsage,
                   AVG(MemoryLimit) AS MemoryLimit,
                   AVG(RxBytes) AS RxBytes,
                   AVG(TxBytes) AS TxBytes,
                   MIN(Created) AS Created
            FROM SwarmServiceStats
            WHERE ((StackId = @StackId AND ServiceName = @ServiceName)
                   OR (StackId IS NULL
                       AND SwarmServiceId IS NULL
                       AND PlatformId = @PlatformId
                       AND DockerServiceId = @DockerServiceId))
              AND Created > @Since
            GROUP BY TaskKey, (Created / @BucketSeconds)
        )
        SELECT SUM(MemoryActive) AS MemoryActive,
               SUM(MemoryCache) AS MemoryCache,
               SUM(CpuUsage) AS CpuUsage,
               SUM(MemoryLimit) AS MemoryLimit,
               SUM(RxBytes) AS RxBytes,
               SUM(TxBytes) AS TxBytes,
               MIN(Created) AS Created
        FROM TaskBuckets
        GROUP BY Bucket
        ORDER BY Bucket
        """;

    private const string DockerServiceStatsSql = """
        WITH TaskBuckets AS (
            SELECT TaskKey,
                   (Created / @BucketSeconds) AS Bucket,
                   AVG(MemoryActive) AS MemoryActive,
                   AVG(MemoryCache) AS MemoryCache,
                   AVG(CpuUsage) AS CpuUsage,
                   AVG(MemoryLimit) AS MemoryLimit,
                   AVG(RxBytes) AS RxBytes,
                   AVG(TxBytes) AS TxBytes,
                   MIN(Created) AS Created
            FROM SwarmServiceStats
            WHERE PlatformId = @PlatformId
              AND DockerServiceId = @DockerServiceId
              AND Created > @Since
            GROUP BY TaskKey, (Created / @BucketSeconds)
        )
        SELECT SUM(MemoryActive) AS MemoryActive,
               SUM(MemoryCache) AS MemoryCache,
               SUM(CpuUsage) AS CpuUsage,
               SUM(MemoryLimit) AS MemoryLimit,
               SUM(RxBytes) AS RxBytes,
               SUM(TxBytes) AS TxBytes,
               MIN(Created) AS Created
        FROM TaskBuckets
        GROUP BY Bucket
        ORDER BY Bucket
        """;

    private static async ValueTask WriteNullableAsync<T>(
        NpgsqlBinaryImporter importer,
        T? value,
        NpgsqlDbType type,
        CancellationToken cancellationToken)
        where T : struct
    {
        if (value.HasValue)
            await importer.WriteAsync(value.Value, type, cancellationToken);
        else
            await importer.WriteNullAsync(cancellationToken);
    }
}
