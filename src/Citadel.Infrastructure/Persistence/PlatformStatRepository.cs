using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class PlatformStatRepository(IDbConnection db, Func<IDbTransaction> tx) : IPlatformStatRepository 
{
    public async Task<IEnumerable<PlatformStat>> GetStatsAggregatedLast24HoursAsync(Guid platformId, CancellationToken cancellationToken)
        => await GetStatsAggregatedAsync(platformId, 24, cancellationToken);

    public async Task<IEnumerable<PlatformStat>> GetStatsAggregatedAsync(Guid platformId, int hours, CancellationToken cancellationToken)
    {
        // We don't retrieve the full stats, but rather aggregate them to reduce the amount of data transferred and processed.
        const string sql = """
            SELECT 
                @PlatformId AS PlatformId,
                MIN(Created) AS Created, 
                AVG(CpuUsage) AS CpuUsage, 
                AVG(MemoryUsage) AS MemoryUsage,
                AVG(RxBytes) AS RxBytes,
                AVG(TxBytes) AS TxBytes,
                ROUND(AVG(DiskUsedBytes))::bigint AS DiskUsedBytes,
                ROUND(AVG(DiskTotalBytes))::bigint AS DiskTotalBytes,
                AVG(DiskUsage) AS DiskUsage
            FROM PlatformStats
            WHERE PlatformId = @PlatformId 
              AND Created > @Since
            GROUP BY (Created / @BucketSeconds)
            ORDER BY MIN(Created)
        """;

        var normalizedHours = Math.Clamp(hours, 1, 72);
        var bucketSeconds = normalizedHours > 24 ? 300 : 60;
        var since = DateTimeOffset.UtcNow.AddHours(-normalizedHours).ToUnixTimeSeconds();
        var result = await db.QueryAsync<PlatformStatDto>(sql, new
        {
            PlatformId = platformId,
            Since = since,
            BucketSeconds = bucketSeconds
        }, transaction: tx());
        
        return result.ToDomain();
    }

    public Task<int> BulkInsertAsync(IEnumerable<PlatformStat> stats, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO PlatformStats
            (Id, PlatformId, Created, CpuUsage, MemoryUsage, RxBytes, TxBytes, DiskUsedBytes, DiskTotalBytes, DiskUsage)
            VALUES
            {0}
        """;
        var parameters = new DynamicParameters();
        var valueRows = new List<string>();
        int i = 0;
        foreach (var stat in stats)
        {
            valueRows.Add($"(@Id{i}, @PlatformId{i}, @Created{i}, @CpuUsage{i}, @MemoryUsage{i}, @RxBytes{i}, @TxBytes{i}, @DiskUsedBytes{i}, @DiskTotalBytes{i}, @DiskUsage{i})");
            parameters.Add($"Id{i}", stat.Id);
            parameters.Add($"PlatformId{i}", stat.PlatformId);
            parameters.Add($"Created{i}", stat.Created);
            parameters.Add($"CpuUsage{i}", stat.CpuUsage);
            parameters.Add($"MemoryUsage{i}", stat.MemoryUsage);
            parameters.Add($"RxBytes{i}", stat.RxBytes);
            parameters.Add($"TxBytes{i}", stat.TxBytes);
            parameters.Add($"DiskUsedBytes{i}", stat.DiskUsedBytes);
            parameters.Add($"DiskTotalBytes{i}", stat.DiskTotalBytes);
            parameters.Add($"DiskUsage{i}", stat.DiskUsage);
            i++;
        }
        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(finalSql, parameters, transaction: tx());
    }


    public async Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        var p = new { CreatedBefore = createdBeforeEpochSeconds };

        var countstats = "SELECT COUNT(*) from  PlatformStats WHERE Created < @CreatedBefore";
        var totalCount = await db.QuerySingleAsync<int>(countstats, p, transaction: tx());

        const string sql = "DELETE FROM PlatformStats WHERE Created < @CreatedBefore";
        await db.ExecuteAsync(sql, p, transaction: tx());
        return totalCount;
    }
}
