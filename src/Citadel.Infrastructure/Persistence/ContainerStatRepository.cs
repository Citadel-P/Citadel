using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Npgsql;
using NpgsqlTypes;

namespace Infrastructure.Persistence;

internal class ContainerStatRepository(IDbConnection db, Func<IDbTransaction> tx) : IContainerStatRepository 
{
    public async Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string dockerContainerId, CancellationToken cancellationToken)
        => await GetStatsAggregatedAsync(dockerContainerId, 24, cancellationToken);

    public async Task<IEnumerable<ContainerStat>> GetStatsAggregatedAsync(string dockerContainerId, int hours, CancellationToken cancellationToken)
    {
        // We don't retrieve the full stats, but rather aggregate them to reduce the amount of data transferred and processed.
        const string sql = """
            SELECT 
              C.Id AS ContainerId,
              MIN(S.Created) AS Created, 
              AVG(S.CpuUsage) AS CpuUsage, 
              AVG(S.MemoryActive) AS MemoryActive,
              AVG(S.MemoryCache) AS MemoryCache,
              AVG(S.MemoryLimit) AS MemoryLimit,
              AVG(S.RxBytes) AS RxBytes,
              AVG(S.TxBytes) AS TxBytes
            FROM ContainerStats S
            INNER JOIN Containers C ON C.Id = S.ContainerId 
            WHERE C.DockerContainerId LIKE @DockerContainerIdPrefix || '%'
              AND S.Created > @Since
            GROUP BY C.Id, (S.Created / @BucketSeconds)
            ORDER BY MIN(S.Created)
            """;
      
        var normalizedHours = Math.Clamp(hours, 1, 72);
        var bucketSeconds = normalizedHours > 24 ? 300 : 60;
        var since = DateTimeOffset.UtcNow.AddHours(-normalizedHours).ToUnixTimeSeconds();
        var result = await db.QueryAsync<ContainerStatDto>(sql, new
        {
            DockerContainerIdPrefix = dockerContainerId,
            Since = since,
            BucketSeconds = bucketSeconds
        }, transaction: tx());

        return result?.ToDomain() ?? [];
    }

    public async Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        const string sql = """
            WITH expired AS (
                SELECT ctid
                FROM ContainerStats
                WHERE Created < @CreatedBefore
                ORDER BY Created
                LIMIT 5000
            )
            DELETE FROM ContainerStats stats
            USING expired
            WHERE stats.ctid = expired.ctid
            """;

        return await db.ExecuteAsync(
            sql,
            new { CreatedBefore = createdBeforeEpochSeconds },
            transaction: tx());
    }

    public async Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken)
    {
        _ = tx();
        if (db is not NpgsqlConnection connection)
            throw new InvalidOperationException("Container statistics require a PostgreSQL connection.");

        await using var importer = await connection.BeginBinaryImportAsync(
            """
            COPY ContainerStats
                (Id, ContainerId, Created, MemoryActive, MemoryCache, CpuUsage, MemoryLimit, RxBytes, TxBytes)
            FROM STDIN (FORMAT BINARY)
            """,
            cancellationToken);

        var count = 0;
        foreach (var stat in stats)
        {
            await importer.StartRowAsync(cancellationToken);
            await importer.WriteAsync(stat.Id, NpgsqlDbType.Uuid, cancellationToken);
            await importer.WriteAsync(stat.ContainerId, NpgsqlDbType.Uuid, cancellationToken);
            await WriteNullableAsync(importer, stat.Created, NpgsqlDbType.Bigint, cancellationToken);
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
