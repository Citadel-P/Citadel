using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;

namespace Infrastructure.Persistence;

internal class ContainerStatRepository(IDbConnection db, Func<IDbTransaction> tx) : IContainerStatRepository 
{
    public async Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string containerId, CancellationToken cancellationToken)
    {
        // We don't retrieve the full stats, but rather aggregate them to reduce the amount of data transferred and processed.
        const string sql = """
            SELECT 
              MIN(S.Created) AS Created, 
              AVG(S.CpuUsage) AS CpuUsage, 
              AVG(S.MemoryActive) AS MemoryActive,
              AVG(S.MemoryCache) AS MemoryCache,
              AVG(S.MemoryLimit) AS MemoryLimit,
              AVG(S.RxBytes) AS RxBytes,
              AVG(S.TxBytes) AS TxBytes
            FROM ContainerStats S
            INNER JOIN Containers C on C.Id = S.ContainerId 
            WHERE C.ContainerId LIKE @ContainerIdPrefix || '%'
              AND S.Created > @Last24h
            GROUP BY strftime('%Y-%m-%d %H:%M', datetime(S.Created, 'unixepoch'))
            ORDER BY S.Created
            """;
      
        var last24h = DateTimeOffset.UtcNow.AddHours(-24).ToUnixTimeSeconds();
        var result = await db.QueryAsync<ContainerStatDto>(sql, new { ContainerIdPrefix = containerId, Last24h = last24h }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM ContainerStats WHERE Created < @CreatedBefore";
        return db.ExecuteAsync(sql, new { CreatedBefore = createdBeforeEpochSeconds }, transaction: tx());
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
            parameters.Add($"Id{i}", stat.Id.Format());
            parameters.Add($"ContainerId{i}", stat.ContainerId.Format());
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
