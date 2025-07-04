using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class ContainerStatRepository(IDbConnection db, IDbTransaction tx) : IContainerStatRepository 
{
    public async Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string containerId, CancellationToken cancellationToken)
    {
        // We don't retrieve the full stats, but rather aggregate them to reduce the amount of data transferred and processed.
        var sql = """
            SELECT 
              MIN(Created) AS Created, 
              AVG(CpuUsage) AS CpuUsage, 
              AVG(MemoryUsage) AS MemoryUsage
              AVG(MemoryLimit) AS MemoryLimit
              AVG(RxBytes) AS RxBytes
              AVG(TxBytes) AS TxBytes
            FROM ContainerStats
            WHERE ContainerId LIKE @ContainerIdPrefix || '%'
              AND Created > @Last24h
            GROUP BY strftime('%Y-%m-%d %H:%M', datetime(Created, 'unixepoch'))
            ORDER BY Created
            """;

        var last24h = DateTimeOffset.UtcNow.AddHours(-24).ToUnixTimeSeconds();
        var result = await db.QueryAsync<ContainerStatDto>(
            new CommandDefinition(sql, new { ContainerId = containerId, Last24h = last24h }, transaction: tx, cancellationToken: cancellationToken));
        return result?.ToDomain() ?? [];
    }

    public Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM ContainerStats WHERE Created < @CreatedBefore";
        var command = new CommandDefinition(sql, new { CreatedBefore = createdBeforeEpochSeconds }, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(command);
    }

    public Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken)
    {
        const string sql = """  
            INSERT INTO ContainerStats
            (Id, ContainerId, Created, MemoryUsage, CpuUsage, MemoryLimit, RxBytes, TxBytes)
            VALUES
            {0}   
         """;

        var parameters = new DynamicParameters();
        var valueRows = new List<string>();
        int i = 0;

        foreach (var stat in stats)
        {
            valueRows.Add($"(@Id{i}, @ContainerId{i}, @Created{i}, @MemoryUsage{i}, @CpuUsage{i}, @MemoryLimit{i}, @RxBytes{i}, @TxBytes{i})");
            parameters.Add($"Id{i}", stat.Id);
            parameters.Add($"ContainerId{i}", stat.ContainerId);
            parameters.Add($"Created{i}", stat.Created);
            parameters.Add($"MemoryUsage{i}", stat.MemoryUsage);
            parameters.Add($"CpuUsage{i}", stat.CpuUsage);
            parameters.Add($"MemoryLimit{i}", stat.MemoryLimit);
            parameters.Add($"RxBytes{i}", stat.RxBytes);
            parameters.Add($"TxBytes{i}", stat.TxBytes);
            i++;
        }

        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(new CommandDefinition(finalSql, parameters, transaction: tx, cancellationToken: cancellationToken));
    }
}
