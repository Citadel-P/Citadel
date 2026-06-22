using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

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
        var p = new { CreatedBefore = createdBeforeEpochSeconds };

        var countstats = "SELECT COUNT(*) from  ContainerStats WHERE Created < @CreatedBefore";
        var totalCount = await db.QuerySingleAsync<int>(countstats, p, transaction: tx());

        const string sql = "DELETE FROM ContainerStats WHERE Created < @CreatedBefore";
        await db.ExecuteAsync(sql, new { CreatedBefore = createdBeforeEpochSeconds }, transaction: tx());

        return totalCount;
    }

    public Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken)
    {
        const string sql = """  
            INSERT INTO ContainerStats
            (Id, ContainerId, Created, MemoryActive, MemoryCache, CpuUsage, MemoryLimit, RxBytes, TxBytes)
            VALUES
            {0}   
         """;

        var parameters = new DynamicParameters();
        var valueRows = new List<string>();
        int i = 0;

        foreach (var stat in stats)
        {
            valueRows.Add($"(@Id{i}, @ContainerId{i}, @Created{i}, @MemoryActive{i}, @MemoryCache{i}, @CpuUsage{i}, @MemoryLimit{i}, @RxBytes{i}, @TxBytes{i})");
            parameters.Add($"Id{i}", stat.Id);
            parameters.Add($"ContainerId{i}", stat.ContainerId);
            parameters.Add($"Created{i}", stat.Created);
            parameters.Add($"MemoryActive{i}", stat.MemoryActive);
            parameters.Add($"MemoryCache{i}", stat.MemoryCache);
            parameters.Add($"CpuUsage{i}", stat.CpuUsage);
            parameters.Add($"MemoryLimit{i}", stat.MemoryLimit);
            parameters.Add($"RxBytes{i}", stat.RxBytes);
            parameters.Add($"TxBytes{i}", stat.TxBytes);
            i++;
        }

        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(finalSql, parameters, transaction: tx());
    }
}
