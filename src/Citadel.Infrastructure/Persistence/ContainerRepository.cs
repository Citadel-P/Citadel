using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class ContainerRepository(IDbConnection db, Func<IDbTransaction> tx) : IContainerRepository 
{
    public async Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Containers
            WHERE PlatformId = @PlatformId
            ORDER BY Created DESC
            """;
        var result = await db.QueryAsync<ContainerDto>(sql, new { PlatformId = platformId.Format() }, tx());
        return result.ToDomain();
    }

    [DapperAot(false)]
    public async Task<IEnumerable<PlatformContainersInfo>?> GetPlatformsByContainerIdsAsync(IEnumerable<string> containerIds, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForStrings("ContainerId", containerIds);
        string sql = $"""
            SELECT 
                Containers.ContainerId, 
                Platforms.Address, 
                Platforms.ConnectorType
            FROM Containers
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            WHERE Containers.ContainerId IN ({clause})
            """;

        var flatRows = await db.QueryAsync<PlatformContainerInfoDto>(sql, parameters, transaction: tx());

        if (!flatRows.Any())
            return null;
            
        return [.. flatRows
            .GroupBy(r => new { r.Address, r.ConnectorType })
            .Select(g => new PlatformContainersInfo(
                Address: g.Key.Address,
                ConnectorType: Enum.Parse<PlatformConnectorType>(g.Key.ConnectorType),
                ContainerIds: [.. g.Select(x => x.ContainerId).Distinct()]
            ))];
    }

    public async Task<Container?> GetByIdAsync(string containerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers
            WHERE ContainerId LIKE @ContainerIdPrefix || '%'
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(sql, new { ContainerIdPrefix = containerId }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<ContainerInfo?> GetContainerInfoAsync(string containerId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT C.Id, C.Name, C.ContainerId, P.Id as PlatformId, P.Name as PlatformName 
            FROM Containers C
            LEFT JOIN Platforms P ON C.PlatformId = P.Id
            WHERE C.ContainerId LIKE @ContainerIdPrefix
            LIMIT 1
            """;
        return db.QuerySingleOrDefaultAsync<ContainerInfo>(sql, new { ContainerIdPrefix = containerId + '%' }, transaction: tx());
    }

    public Task<int> BulkInsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Containers (
                Id, PlatformId, ContainerId, Name, Image, Created, Updated, State, Stack, Ports
            ) VALUES {0}
        """;

        var valueRows = new List<string>();
        var parameters = new DynamicParameters();
        int i = 0;

        foreach (var c in containers)
        {
            valueRows.Add(
                $"(@Id{i}, @PlatformId{i}, @ContainerId{i}, @Name{i}, @Image{i}, @Created{i}, @Updated{i}, @State{i}, @Stack{i}, @Ports{i})"
            );
            parameters.Add($"Id{i}", c.Id.Format());
            parameters.Add($"PlatformId{i}", c.PlatformId.Format());
            parameters.Add($"ContainerId{i}", c.ContainerId);
            parameters.Add($"Name{i}", c.Name);
            parameters.Add($"Image{i}", c.Image);
            parameters.Add($"Created{i}", c.Created);
            parameters.Add($"Updated{i}", c.Updated);
            parameters.Add($"State{i}", EnumFormatter<ContainerStateStatus>.GetValue(c.State));
            parameters.Add($"Stack{i}", c.Stack);
            parameters.Add($"Ports{i}", JsonSerializer.Serialize( c.Ports?.ToList() ?? [], ContainerPortsContext.Default.IReadOnlyCollectionContainerPort));
            i++;
        }

        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(finalSql, parameters, transaction: tx());
    }

    [DapperAot(false)]
    public async Task<IEnumerable<Container>?> GetAllWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT c.*, s.*
        FROM Containers c
        LEFT JOIN ContainerStats s ON s.ContainerId = c.Id
          AND s.Id = (
              SELECT Id FROM ContainerStats 
              WHERE ContainerId = c.Id 
              ORDER BY Created DESC 
              LIMIT 1
          )
        WHERE c.PlatformId = @PlatformId
        ORDER BY c.Created DESC;
        """;

        var lookup = new Dictionary<string, ContainerDto>();
        var result = await db.QueryAsync<ContainerDto, ContainerStatDto, ContainerDto>(
            sql,
            (container, stat) =>
            {
                if (!lookup.TryGetValue(container.Id, out var existing))
                {
                    existing = container;
                    lookup[container.Id] = existing;
                }

                if (stat != null)
                {
                    existing.Stats?.Add(stat);
                }

                return existing;
            },
            param: new { PlatformId = platformId.Format() },
            splitOn: "Id"
        );

        return [.. lookup.Values.ToDomain()];
    }

    public Task<int> AddAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Containers (
                Id, PlatformId, ContainerId, Name, Image, Created, Updated, State, Stack, Ports
            ) VALUES (
                @Id, @PlatformId, @ContainerId, @Name, @Image, @Created, @Updated, @State, @Stack, @Ports
            )
        """;
        return db.ExecuteAsync(sql, new 
        {
            Id = container.Id.Format(),
            PlatformId = container.PlatformId.Format(),
            ContainerId = container.ContainerId,
            Name = container.Name,
            Image = container.Image,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.Stack,
            Ports = JsonSerializer.Serialize(container.Ports?.ToList() ?? [], ContainerPortsContext.Default.IReadOnlyCollectionContainerPort)
        }, transaction: tx());
    }

    [DapperAot(false)]
    public Task<int> UpdateContainersStateAsync(IEnumerable<Guid> ids, ContainerStateStatus state, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForGuids("Id", ids);
        string sql = $"""
            UPDATE Containers
            SET State = @State, Updated = @Updated
            WHERE Id IN ({clause})
        """;

        parameters.Add("State", state);
        parameters.Add("Updated", DateTimeOffset.UtcNow.ToUnixTimeSeconds());
        return db.ExecuteAsync(sql, parameters, transaction: tx());
    }

    public Task<int> UpdateContainerAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET Name = @Name, Image = @Image, Updated = @Updated, State = @State, Stack = @Stack, Ports = @Ports, Created = @Created
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
        
            Id = container.Id.Format(),
            PlatformId = container.PlatformId.Format(),
            ContainerId = container.ContainerId,
            Name = container.Name,
            Image = container.Image,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.Stack,
            Ports = JsonSerializer.Serialize(container.Ports?.ToList() ?? [], ContainerPortsContext.Default.IReadOnlyCollectionContainerPort)
        }, transaction: tx());
    }

    [DapperAot(false)]
    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForGuids("Id", ids);
        string sql = $"DELETE FROM Containers WHERE Id IN ({clause})";
        return db.ExecuteAsync(sql, parameters, transaction: tx());
    }

}
