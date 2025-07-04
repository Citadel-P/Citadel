using System.Data;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;

namespace Infrastructure.Persistence;

internal class ContainerRepository(IDbConnection db, IDbTransaction tx) : IContainerRepository 
{
    public async Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers
            WHERE PlatformId = @PlatformId
            ORDER BY Created DESC
            """;
        var result = await db.QueryAsync<ContainerDto>(
            new CommandDefinition(sql, new { PlatformId = platformId }, transaction: tx, cancellationToken: cancellationToken));
        return result.ToDomain();
    }

    public async Task<IEnumerable<PlatformContainersInfo>?> GetPlatformsByContainerIdsAsync(IEnumerable<string> containerIds, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
            
                Containers.ContainerId, 
                Platforms.Address, 
                Platforms.ConnectorType
            FROM Containers
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            WHERE Containers.ContainerId IN @Ids
            """;

        var flatRows = await db.QueryAsync<(string ContainerId, string Address, PlatformConnectorType ConnectorType)>(
            new CommandDefinition(sql, new { Ids = containerIds }, transaction: tx, cancellationToken: cancellationToken));

        if (!flatRows.Any())
            return null;
            
        return [.. flatRows
            .GroupBy(r => new { r.Address, r.ConnectorType })
            .Select(g => new PlatformContainersInfo(
                Address: g.Key.Address,
                ConnectorType: g.Key.ConnectorType,
                ContainerIds: [.. g.Select(x => x.ContainerId).Distinct()]
            ))];
    }

    public async Task<Container?> GetByIdAsync(string containerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers
            WHERE ContainerId LIKE @ContainerIdPrefix || '%'
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(
            new CommandDefinition(sql, new { ContainerIdPrefix = containerId }, transaction: tx, cancellationToken: cancellationToken));
        return result?.ToDomain();
    }

    public Task<(string? Address, Guid? PlatformId, PlatformConnectorType? ConnectorType)> GetPlatformByContainerIdAsync(string containerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT 
                Platforms.Address,
                Containers.PlatformId,
                Platforms.ConnectorType
            FROM Containers
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            WHERE Containers.ContainerId LIKE @ContainerIdPrefix || '%'
            LIMIT 1
            """;

        return db.QuerySingleOrDefaultAsync<(string? Address, Guid? PlatformId, PlatformConnectorType? ConnectorType)>(
            new CommandDefinition(sql, new { ContainerIdPrefix = containerId }, transaction: tx, cancellationToken: cancellationToken));
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
            parameters.Add($"Id{i}", c.Id);
            parameters.Add($"PlatformId{i}", c.PlatformId);
            parameters.Add($"ContainerId{i}", c.ContainerId);
            parameters.Add($"Name{i}", c.Name);
            parameters.Add($"Image{i}", c.Image);
            parameters.Add($"Created{i}", c.Created);
            parameters.Add($"Updated{i}", c.Updated);
            parameters.Add($"State{i}", c.State);
            parameters.Add($"Stack{i}", c.Stack);
            parameters.Add($"Ports{i}", c.Ports?.ToList() ?? []);
            i++;
        }

        var finalSql = string.Format(sql, string.Join(", ", valueRows));
        return db.ExecuteAsync(new CommandDefinition(finalSql, parameters, transaction: tx, cancellationToken: cancellationToken));
    }

    public async Task<IEnumerable<Container>?> GetAllWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        var sql = """
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

        var lookup = new Dictionary<Guid, ContainerDto>();
        var command = new CommandDefinition(
            sql,
            parameters: new { PlatformId = platformId },
            cancellationToken: cancellationToken
        );
        var result = await db.QueryAsync<ContainerDto, ContainerStatDto, ContainerDto>(
            command,
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
        var command = new CommandDefinition(sql, container, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(command);
    }

    public Task<int> UpdateContainersStateAsync(IEnumerable<Guid> ids, ContainerStateStatus state, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET State = @State, Updated = @Updated
            WHERE Id IN @Ids
        """;
        var command = new CommandDefinition(sql, new { State = state, Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds(), Ids = ids }, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(command);
    }

    public Task<int> UpdateContainerAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET Name = @Name, Image = @Image, Updated = @Updated, State = @State, Stack = @Stack, Ports = @Ports, Created = @Created
            WHERE Id = @Id
        """;
        var command = new CommandDefinition(sql, container, transaction: tx, cancellationToken: cancellationToken);
        return db.ExecuteAsync(command);
    }

    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM Containers
            WHERE Id IN @Ids
        """;
        return db.ExecuteAsync(new CommandDefinition(sql, new { Ids = ids }, transaction: tx, cancellationToken: cancellationToken));
    }

}
