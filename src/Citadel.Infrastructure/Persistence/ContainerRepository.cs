using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using System.Data;
using System.Text.Json;
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

    public async Task<IEnumerable<Container>> GetByIdAsync(IEnumerable<string> dockerContainersId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE DockerContainerId IN (SELECT value FROM json_each(@DockerContainersId))
            LIMIT 1
            """;
        var result = await db.QueryAsync<ContainerDto>(sql, new 
        {
            DockerContainersId = JsonSerializer.Serialize(dockerContainersId, DeploymentJsonContext.Default.IEnumerableString) 
        }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<IEnumerable<Container>> GetStuckContainersAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE ControlState = @ControlState
              AND ControlStartedAt IS NOT NULL
              AND ControlStartedAt < @TimeoutThreshold
            """;
        var timeoutThreshold = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s;
        var result = await db.QueryAsync<ContainerDto>(sql, new 
        { 
            ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
            TimeoutThreshold = timeoutThreshold
        }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<Container?> GetByDeploymentIdAsync(Guid deploymentId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE DeploymentId = @DeploymentId
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(sql, new { DeploymentId = deploymentId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>> GetByDeploymentIdsAsync(IEnumerable<Guid> deploymentIds, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT *
        FROM Containers c
        WHERE c.DeploymentId IN (
            SELECT value FROM json_each(@DeploymentIds)
        )
        LIMIT 1
        """;

        var result = await db.QueryAsync<ContainerDto>(sql, 
            new { DeploymentIds = JsonSerializer.Serialize(deploymentIds, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx()
        );

        return result.ToDomain();
    }

    public async Task<Container?> GetContainerInfoAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT c.*,
                i.Id as Image_ImageId,
                i.Name as Image_Name,
                i.Tags as Image_Tags,
                i.DockerImageId as Image_DockerImageId,
                i.Size as Image_Size,
                i.Containers as Image_Containers,
                i.PlatformId as Image_PlatformId,
                i.CreatedAt as Image_CreatedAt,               
                i.UpdatedAt as Image_UpdatedAt,
                i.RegistryId as Image_RegistryId,
                d.Id as Deployment_DeploymentId,
                d.Name as Deployment_DeploymentName,
                d.Status as Deployment_DeploymentStatus
            FROM Containers c
            LEFT JOIN Images i ON c.ImageId = i.Id
            LEFT JOIN Deployments d ON c.DeploymentId = d.Id
            WHERE DockerContainerId LIKE @DockerContainerIdPrefix || '%'
            LIMIT 1
            """;
       
        var result = await db.QuerySingleOrDefaultAsync<ContainerWithImageDto>(sql, new { DockerContainerIdPrefix = dockerContainerId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>?> GetContainersInfoAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT c.*,
            i.Id as Image_ImageId,
            i.Name as Image_Name,
            i.Tags as Image_Tags,
            i.DockerImageId as Image_DockerImageId,
            i.Size as Image_Size,
            i.Containers as Image_Containers,
            i.PlatformId as Image_PlatformId,
            i.CreatedAt as Image_CreatedAt,
            i.UpdatedAt as Image_UpdatedAt,
            i.RegistryId as Image_RegistryId,
            s.Created as Stat_Created,
            s.MemoryActive as Stat_MemoryActive,
            s.MemoryCache as Stat_MemoryCache, 
            s.CpuUsage as Stat_CpuUsage, 
            s.MemoryLimit as Stat_MemoryLimit, 
            s.RxBytes as Stat_RxBytes, 
            s.TxBytes as Stat_TxBytes,
            d.Id as Deployment_DeploymentId,
            d.Name as Deployment_DeploymentName,
            d.status as Deployment_DeploymentStatus
        FROM Containers c
        LEFT JOIN Images i ON c.ImageId = i.Id
        LEFT JOIN Deployments d ON c.DeploymentId = d.Id
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
                Id, PlatformId, DockerContainerId, Name, DockerImageId, Created, Updated, State, Stack, Ports, ImageId, deploymentId
            ) VALUES (
                @Id, @PlatformId, @DockerContainerId, @Name, @DockerImageId, @Created, @Updated, @State, @Stack, @Ports, @ImageId, @DeploymentId
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
            DeploymentId = container.DeploymentId?.Format(),
            Ports = JsonSerializer.Serialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding)
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET Name = @Name, DockerImageId = @DockerImageId, Updated = @Updated, State = @State, Stack = @Stack, Ports = @Ports, Created = @Created, ImageId = @ImageId, DeploymentId = @DeploymentId, PlatformId = @PlatformId
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {

            Id = container.Id.Format(),
            PlatformId = container.PlatformId.Format(),
            ImageId = container.ImageId?.Format(),
            DeploymentId = container.DeploymentId?.Format(),
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
        const string sql = """
        UPDATE Containers
        SET State   = @State,
            Updated = @Updated
        WHERE Id IN (
            SELECT value FROM json_each(@Ids)
        )
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid),
                State = state,
                Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            },
            transaction: tx()
        );
    }

    public Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, CancellationToken cancellationToken)
    {
        var conditions = new List<string>
        {
            "Id = @Id"
        };

        if (checkRowVersion == true)
            conditions.Add("RowVersion = @RowVersion");

        var sql = $"""
            UPDATE Containers
            SET
                ControlState = @State,
                ControlStartedAt = @StartedAt,
                RowVersion = RowVersion + 1
            WHERE {string.Join(" AND ", conditions)}
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id.Format(),
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                RowVersion = rowVersion,
                StartedAt = startedAt
            },
            transaction: tx()
        );
    }

    public Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Containers
        WHERE Id IN (
            SELECT value FROM json_each(@Ids)
        )
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx()
        );
    }
}
