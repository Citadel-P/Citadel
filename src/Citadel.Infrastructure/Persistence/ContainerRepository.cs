using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
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
            ORDER BY 
                Created DESC,
                Name ASC
            """;
        var result = await db.QueryAsync<ContainerDto>(sql, new { PlatformId = platformId.Format() }, tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<Container?> GetByIdAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE DockerContainerId LIKE @DockerContainerIdPrefix || '%'
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(sql, new { DockerContainerIdPrefix = dockerContainerId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<Container?> GetContainerWithImageByIdAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT c.*,
                i.Id as Image_ImageId,
                i.Name as Image_Name,
                i.Tag as Image_Tag,
                i.DockerImageId as Image_DockerImageId,
                i.Size as Image_Size,
                i.Containers as Image_Containers,
                i.PlatformId as Image_PlatformId,
                i.CreatedAt as Image_CreatedAt,
                i.IsUpToDate as Image_IsUpToDate,
                i.UpdatedAt as Image_UpdatedAt,
                i.RegistryId as Image_RegistryId
            FROM Containers c
            LEFT JOIN Images i ON c.ImageId = i.Id
            WHERE DockerContainerId LIKE @DockerContainerIdPrefix || '%'
            LIMIT 1
            """;
       
        var result = await db.QuerySingleOrDefaultAsync<ContainerWithImageDto>(sql, new { DockerContainerIdPrefix = dockerContainerId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>?> GetAllWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT c.*,
            i.Id as Image_ImageId,
            i.Name as Image_Name,
            i.Tag as Image_Tag,
            i.DockerImageId as Image_DockerImageId,
            i.Size as Image_Size,
            i.Containers as Image_Containers,
            i.PlatformId as Image_PlatformId,
            i.CreatedAt as Image_CreatedAt,
            i.IsUpToDate as Image_IsUpToDate,
            i.UpdatedAt as Image_UpdatedAt,
            i.RegistryId as Image_RegistryId,
            s.Created as Stat_Created,
            s.MemoryActive as Stat_MemoryActive,
            s.MemoryCache as Stat_MemoryCache, 
            s.CpuUsage as Stat_CpuUsage, 
            s.MemoryLimit as Stat_MemoryLimit, 
            s.RxBytes as Stat_RxBytes, 
            s.TxBytes as Stat_TxBytes
        FROM Containers c
        LEFT JOIN Images i ON c.ImageId = i.Id
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

        var result = await db.QueryAsync<ContainerWithLastStatDto>(sql, new { PlatformId = platformId.Format() }, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> AddAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Containers (
                Id, PlatformId, DockerContainerId, Name, DockerImageId, Created, Updated, State, Stack, Ports, ImageId
            ) VALUES (
                @Id, @PlatformId, @DockerContainerId, @Name, @DockerImageId, @Created, @Updated, @State, @Stack, @Ports, @ImageId
            )
        """;
        return db.ExecuteAsync(sql, new 
        {
            Id = container.Id.Format(),
            PlatformId = container.PlatformId.Format(),
            DockerContainerId = container.DockerContainerId,
            DockerImageId = container.DockerImageId,
            Name = container.Name,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.Stack,
            ImageId = container.ImageId?.Format(),
            Ports = JsonSerializer.Serialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding)
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET Name = @Name, DockerImageId = @DockerImageId, Updated = @Updated, State = @State, Stack = @Stack, Ports = @Ports, Created = @Created, ImageId = @ImageId
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {

            Id = container.Id.Format(),
            PlatformId = container.PlatformId.Format(),
            ImageId = container.ImageId?.Format(),
            Name = container.Name,
            Image = container.Image,
            DockerImageId = container.DockerImageId,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.Stack,
            Ports = JsonSerializer.Serialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding)
        }, transaction: tx());
    }

    public Task<int> BulkUpsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken)
    {
        const string sql = """
        INSERT INTO Containers (Id, PlatformId, DockerContainerId, Name, DockerImageId, Created, Updated, State, Stack, Ports, ImageId)
        VALUES (@Id, @PlatformId, @DockerContainerId, @Name, @DockerImageId, @Created, @Updated, @State, @Stack, @Ports, @ImageId)
        ON CONFLICT(Id) DO UPDATE SET
            Name = excluded.Name,
            DockerImageId = excluded.DockerImageId,
            ImageId = excluded.ImageId,
            Created = excluded.Created,
            Updated = excluded.Updated,
            State = excluded.State,
            Stack = excluded.Stack,
            Ports = excluded.Ports;
    """;

        return db.ExecuteAsync(sql, containers.Select(c => new
        {
            Id = c.Id.Format(),
            PlatformId = c.PlatformId.Format(),
            DockerContainerId = c.DockerContainerId,
            Name = c.Name,
            DockerImageId = c.DockerImageId,
            Created = c.Created,
            Updated = c.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(c.State),
            Stack = c.Stack,
            ImageId = c.ImageId?.Format(),
            Ports = JsonSerializer.Serialize(
                c.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding
            )
        }), transaction: tx());
    }

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

    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        var (clause, parameters) = SqliteInClauseBuilder.BuildInClauseForGuids("Id", ids);
        string sql = $"DELETE FROM Containers WHERE Id IN ({clause})";
        return db.ExecuteAsync(sql, parameters, transaction: tx());
    }
}
