using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;

namespace Infrastructure.Persistence;

internal class PlatformStatRepository(IDbConnection db, Func<IDbTransaction> tx) : IPlatformStatRepository 
{
    public async Task<IEnumerable<PlatformStat>> GetStatsAggregatedLast24HoursAsync(Guid platformId, CancellationToken cancellationToken)
    {
        // We don't retrieve the full stats, but rather aggregate them to reduce the amount of data transferred and processed.
        const string sql = """
            SELECT 
                MIN(Created) AS Created, 
                AVG(CpuUsage) AS CpuUsage, 
                AVG(MemoryUsage) AS MemoryUsage,
                AVG(RxBytes) AS RxBytes,
                AVG(TxBytes) AS TxBytes
            FROM PlatformStats
            WHERE PlatformId = @PlatformId AND Created > @Last24h
            GROUP BY strftime('%Y-%m-%d %H:%M', datetime(Created, 'unixepoch'))
            ORDER BY Created
        """;
        
        var last24h = DateTimeOffset.UtcNow.AddHours(-24).ToUnixTimeSeconds();
        var result = await db.QueryAsync<PlatformStatDto>(sql, new { PlatformId = platformId.Format(), Last24h = last24h }, transaction: tx());
        
        return result.ToDomain();
    }

    public Task<int> BulkInsertAsync(IEnumerable<PlatformStat> stats, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO PlatformStats
            (Id, PlatformId, Created, CpuUsage, MemoryUsage, RxBytes, TxBytes)
            VALUES
            {0}
        """;
        var parameters = new DynamicParameters();
        var valueRows = new List<string>();
        int i = 0;
        foreach (var stat in stats)
        {
            valueRows.Add($"(@Id{i}, @PlatformId{i}, @Created{i}, @CpuUsage{i}, @MemoryUsage{i}, @RxBytes{i}, @TxBytes{i})");
            parameters.Add($"Id{i}", stat.Id.Format());
            parameters.Add($"PlatformId{i}", stat.PlatformId?.Format());
            parameters.Add($"Created{i}", stat.Created);
            parameters.Add($"CpuUsage{i}", stat.CpuUsage);
            parameters.Add($"MemoryUsage{i}", stat.MemoryUsage);
            parameters.Add($"RxBytes{i}", stat.RxBytes);
            parameters.Add($"TxBytes{i}", stat.TxBytes);
            i++;
        }
        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(finalSql, parameters, transaction: tx());
    }


    public Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM PlatformStats WHERE Created < @CreatedBefore";
        return db.ExecuteAsync(sql, new { CreatedBefore = createdBeforeEpochSeconds }, transaction: tx());
    }
}
