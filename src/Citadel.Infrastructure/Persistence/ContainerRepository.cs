using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class ContainerRepository(IDbConnection db, Func<IDbTransaction> tx) : IContainerRepository
{
    public async Task<Container?> GetByIdAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Containers
            WHERE Id = @Id
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(
            sql,
            new { Id = id },
            tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Containers
            WHERE PlatformId = @PlatformId
            ORDER BY 
                Created DESC,
                Name ASC
            """;
        var result = await db.QueryAsync<ContainerDto>(sql, new { PlatformId = platformId }, tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<IEnumerable<Container>> GetByPlatformAndNodeIdAsync(
        Guid platformId,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Containers
            WHERE PlatformId = @PlatformId
              AND DockerNodeId = @DockerNodeId
            ORDER BY
                Created DESC,
                Name ASC
            """;
        var result = await db.QueryAsync<ContainerDto>(
            sql,
            new { PlatformId = platformId, DockerNodeId = dockerNodeId },
            tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<Container?> GetByIdAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        if (Guid.TryParse(dockerContainerId, out var id))
            return await GetByIdAsync(id, cancellationToken);

        var sql = """
            SELECT * FROM Containers c
            WHERE DockerContainerId LIKE @DockerContainerIdPrefix || '%'
            LIMIT 2
            """;
        var result = (await db.QueryAsync<ContainerDto>(sql, new { DockerContainerIdPrefix = dockerContainerId }, transaction: tx())).ToArray();
        return result.Length == 1 ? result[0].ToDomain() : null;
    }

    public async Task<Container?> GetByRuntimeIdentityAsync(
        Guid platformId,
        string? dockerNodeId,
        string dockerContainerId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT * FROM Containers
            WHERE PlatformId = @PlatformId
              AND DockerNodeId IS NOT DISTINCT FROM @DockerNodeId
              AND DockerContainerId = @DockerContainerId
            """;
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(
            sql,
            new { PlatformId = platformId, DockerNodeId = dockerNodeId, DockerContainerId = dockerContainerId },
            tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>> GetByIdsAsync(string[] dockerContainerIds, CancellationToken cancellationToken)
    {
        if (dockerContainerIds.Length == 0)
            return [];

        var ids = new List<Guid>(dockerContainerIds.Length);
        var runtimeIds = new List<string>(dockerContainerIds.Length);
        foreach (var value in dockerContainerIds)
        {
            if (Guid.TryParse(value, out var id))
                ids.Add(id);
            else
                runtimeIds.Add(value);
        }

        var containers = new List<Container>(dockerContainerIds.Length);
        if (ids.Count > 0)
        {
            const string byIdSql = """
                SELECT * FROM Containers
                WHERE Id = ANY(@Ids)
                """;
            var byId = await db.QueryAsync<ContainerDto>(
                byIdSql,
                new { Ids = ids.ToArray() },
                transaction: tx());
            containers.AddRange(byId.ToDomain());
        }

        if (runtimeIds.Count == 0)
            return containers;

        const string sql = """
            SELECT c.*
            FROM unnest(@Ids::text[]) AS requested(DockerContainerIdPrefix)
            JOIN LATERAL (
                SELECT (array_agg(candidate.Id ORDER BY candidate.DockerContainerId))[1] AS Id,
                       COUNT(*) AS MatchCount
                FROM (
                    SELECT c.Id, c.DockerContainerId
                    FROM Containers c
                    WHERE c.DockerContainerId LIKE requested.DockerContainerIdPrefix || '%'
                    ORDER BY c.DockerContainerId
                    LIMIT 2
                ) candidate
            ) resolved ON resolved.MatchCount = 1
            JOIN Containers c ON c.Id = resolved.Id
            """;

        var result = await db.QueryAsync<ContainerDto>(sql, new { Ids = runtimeIds.ToArray() }, transaction: tx());
        containers.AddRange(result.ToDomain());
        return containers;
    }

    public async Task<IEnumerable<Container>> GetStaleByDockerIdsAsync(
        string[] dockerContainerIds,
        Guid[] resolvedContainerIds,
        CancellationToken cancellationToken)
    {
        if (dockerContainerIds.Length == 0)
            return [];

        const string sql = """
            SELECT c.*
            FROM unnest(@Ids::text[]) AS requested(DockerContainerIdPrefix)
            JOIN LATERAL (
                SELECT (array_agg(candidate.Id ORDER BY candidate.DockerContainerId))[1] AS Id,
                       COUNT(*) AS MatchCount
                FROM (
                    SELECT c.Id, c.DockerContainerId
                    FROM Containers c
                    WHERE c.DockerContainerId LIKE requested.DockerContainerIdPrefix || '%'
                      AND c.Id <> ALL(@ResolvedIds)
                    ORDER BY c.DockerContainerId
                    LIMIT 2
                ) candidate
            ) resolved ON resolved.MatchCount = 1
            JOIN Containers c ON c.Id = resolved.Id
            """;

        var result = await db.QueryAsync<ContainerDto>(
            sql,
            new
            {
                Ids = dockerContainerIds,
                ResolvedIds = resolvedContainerIds
            },
            transaction: tx());

        return result?.ToDomain() ?? [];
    }

    public async Task<IEnumerable<Container>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE Id = ANY(@Ids)
            """;
        var result = await db.QueryAsync<ContainerDto>(sql, new
        {
            Ids = ids.ToArray()
        }, transaction: tx());
        return result?.ToDomain() ?? [];
    }

    public async Task<IEnumerable<Container>> GetStuckContainersAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        var sql = """
            SELECT * FROM Containers c
            WHERE ControlState = 'Processing'
              AND ControlStartedAt IS NOT NULL
              AND ControlStartedAt < @TimeoutThreshold
            """;
        var timeoutThreshold = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s;
        var result = await db.QueryAsync<ContainerDto>(sql, new
        {
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
        var result = await db.QuerySingleOrDefaultAsync<ContainerDto>(sql, new { DeploymentId = deploymentId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Container>> GetByDeploymentIdsAsync(IEnumerable<Guid> deploymentIds, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT *
        FROM Containers c
        WHERE c.DeploymentId = ANY(@DeploymentIds)
        """;

        var result = await db.QueryAsync<ContainerDto>(sql,
            new { DeploymentIds = deploymentIds.ToArray() },
            transaction: tx()
        );

        return result.ToDomain();
    }

    public async Task<Container?> GetContainerInfoAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        if (Guid.TryParse(dockerContainerId, out var id))
            return await GetContainerInfoByIdAsync(id, cancellationToken);

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

    public async Task<Container?> GetContainerInfoByIdAsync(Guid id, CancellationToken cancellationToken)
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
                d.Id as Deployment_DeploymentId,
                d.Name as Deployment_DeploymentName,
                d.Status as Deployment_DeploymentStatus
            FROM Containers c
            LEFT JOIN Images i ON c.ImageId = i.Id
            LEFT JOIN Deployments d ON c.DeploymentId = d.Id
            WHERE c.Id = @Id
            """;

        var result = await db.QuerySingleOrDefaultAsync<ContainerWithImageDto>(
            sql,
            new { Id = id },
            transaction: tx());
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
            d.status as Deployment_DeploymentStatus,
            ss.Id as Stack_StackId,
            ss.Name as Stack_StackName,
            sn.Hostname as NodeHostname
        FROM Containers c
        LEFT JOIN Images i ON c.ImageId = i.Id
        LEFT JOIN Deployments d ON c.DeploymentId = d.Id
        LEFT JOIN Stacks ss ON c.StackId = ss.Id
        LEFT JOIN SwarmNodeProjections sn
          ON sn.PlatformId = c.PlatformId
         AND sn.DockerNodeId = c.DockerNodeId
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

        var result = await db.QueryAsync<ContainerWithLastStatDto>(sql, new { PlatformId = platformId }, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> AddAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Containers (
                Id, PlatformId, DockerContainerId, Name, DockerImageId, Created, Updated, State, Stack, Ports, ImageId,
                deploymentId, StackId, IsSystem, SystemRole, HasCitadelOwnershipLabels, IsSwarmTask,
                DockerNodeId, ProjectionObservedAt, ProjectionStaleSince, ProjectionStaleReason
            ) VALUES (
                @Id, @PlatformId, @DockerContainerId, @Name, @DockerImageId, @Created, @Updated, @State, @Stack, @Ports::json, @ImageId,
                CASE WHEN @DeploymentId IS NULL OR EXISTS (SELECT 1 FROM Deployments WHERE Id = @DeploymentId) THEN @DeploymentId ELSE NULL END,
                CASE WHEN @StackId IS NULL OR EXISTS (SELECT 1 FROM Stacks WHERE Id = @StackId) THEN @StackId ELSE NULL END,
                @IsSystem, @SystemRole, @HasCitadelOwnershipLabels, @IsSwarmTask,
                @DockerNodeId, @ProjectionObservedAt, @ProjectionStaleSince, @ProjectionStaleReason
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = container.Id,
            PlatformId = container.PlatformId,
            DockerContainerId = container.DockerContainerId,
            DockerImageId = container.DockerImageId,
            Name = container.Name,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.DockerStack,
            ImageId = container.ImageId,
            DeploymentId = container.DeploymentId,
            StackId = container.StackId,
            IsSystem = container.IsSystem,
            SystemRole = container.SystemRole?.ToString(),
            HasCitadelOwnershipLabels = container.HasCitadelOwnershipLabels,
            IsSwarmTask = container.IsSwarmTask,
            DockerNodeId = container.DockerNodeId,
            ProjectionObservedAt = container.ProjectionObservedAt,
            ProjectionStaleSince = container.ProjectionStaleSince,
            ProjectionStaleReason = container.ProjectionStaleReason,
            Ports = JsonSerializer.Serialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding)
        }, transaction: tx());
    }

    public Task<int> UpdateAsync(Container container, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET Name = @Name,
                DockerImageId = @DockerImageId,
                Updated = @Updated,
                State = @State,
                Stack = @Stack,
                Ports = @Ports::json,
                Created = @Created,
                ImageId = @ImageId,
                DeploymentId = CASE WHEN @DeploymentId IS NULL OR EXISTS (SELECT 1 FROM Deployments WHERE Id = @DeploymentId) THEN @DeploymentId ELSE NULL END,
                PlatformId = @PlatformId,
                StackId = CASE WHEN @StackId IS NULL OR EXISTS (SELECT 1 FROM Stacks WHERE Id = @StackId) THEN @StackId ELSE NULL END,
                IsSystem = @IsSystem,
                SystemRole = @SystemRole,
                HasCitadelOwnershipLabels = @HasCitadelOwnershipLabels,
                IsSwarmTask = @IsSwarmTask,
                DockerNodeId = @DockerNodeId,
                ProjectionObservedAt = @ProjectionObservedAt,
                ProjectionStaleSince = @ProjectionStaleSince,
                ProjectionStaleReason = @ProjectionStaleReason
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {

            Id = container.Id,
            PlatformId = container.PlatformId,
            ImageId = container.ImageId,
            DeploymentId = container.DeploymentId,
            StackId = container.StackId,
            IsSystem = container.IsSystem,
            SystemRole = container.SystemRole?.ToString(),
            HasCitadelOwnershipLabels = container.HasCitadelOwnershipLabels,
            IsSwarmTask = container.IsSwarmTask,
            DockerNodeId = container.DockerNodeId,
            ProjectionObservedAt = container.ProjectionObservedAt,
            ProjectionStaleSince = container.ProjectionStaleSince,
            ProjectionStaleReason = container.ProjectionStaleReason,
            Name = container.Name,
            Image = container.Image,
            DockerImageId = container.DockerImageId,
            Created = container.Created,
            Updated = container.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(container.State),
            Stack = container.DockerStack,
            Ports = JsonSerializer.Serialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding)
        }, transaction: tx());
    }

    public Task<int> TryAssignToDeploymentAsync(
        Guid id,
        Guid platformId,
        string dockerContainerId,
        Guid deploymentId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET DeploymentId = @DeploymentId,
                Updated = @Updated
            WHERE Id = @Id
              AND PlatformId = @PlatformId
              AND DockerContainerId = @DockerContainerId
              AND DeploymentId IS NULL
              AND StackId IS NULL
              AND IsSystem = FALSE
              AND IsSwarmTask = FALSE
              AND HasCitadelOwnershipLabels = FALSE
              AND ControlState <> 'Processing'
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                PlatformId = platformId,
                DockerContainerId = dockerContainerId,
                DeploymentId = deploymentId,
                Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            },
            tx());
    }

    public Task<int> TryAssignComposeProjectToStackAsync(
        Guid platformId,
        string projectName,
        IReadOnlyCollection<Guid> containerIds,
        IReadOnlyCollection<string> dockerContainerIds,
        Guid stackId,
        Guid? orphanedOwnerStackId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET StackId = @StackId,
                Updated = @Updated
            WHERE Id = ANY(@ContainerIds)
              AND DockerContainerId = ANY(@DockerContainerIds)
              AND PlatformId = @PlatformId
              AND Stack = @ProjectName
              AND DeploymentId IS NULL
              AND StackId IS NULL
              AND IsSystem = FALSE
              AND (
                  HasCitadelOwnershipLabels = FALSE
                  OR (
                      @OrphanedOwnerStackId IS NOT NULL
                      AND NOT EXISTS (SELECT 1 FROM Stacks WHERE Id = @OrphanedOwnerStackId)
                  )
              )
              AND ControlState <> 'Processing'
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                PlatformId = platformId,
                ProjectName = projectName,
                ContainerIds = containerIds.ToArray(),
                DockerContainerIds = dockerContainerIds.ToArray(),
                StackId = stackId,
                OrphanedOwnerStackId = orphanedOwnerStackId,
                Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            },
            tx());
    }

    public Task<int> BulkUpsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken)
    {
        const string sql = """
        INSERT INTO Containers (
            Id, PlatformId, DockerContainerId, Name, DockerImageId, Created, Updated, State, Stack, Ports, ImageId,
            StackId, IsSystem, SystemRole, HasCitadelOwnershipLabels, IsSwarmTask,
            DockerNodeId, ProjectionObservedAt, ProjectionStaleSince, ProjectionStaleReason)
        VALUES (
            @Id, @PlatformId, @DockerContainerId, @Name, @DockerImageId, @Created, @Updated, @State, @Stack, @Ports::json, @ImageId,
            CASE WHEN @StackId IS NULL OR EXISTS (SELECT 1 FROM Stacks WHERE Id = @StackId) THEN @StackId ELSE NULL END,
            @IsSystem, @SystemRole, @HasCitadelOwnershipLabels, @IsSwarmTask,
            @DockerNodeId, @ProjectionObservedAt, @ProjectionStaleSince, @ProjectionStaleReason
        )
        ON CONFLICT(Id) DO UPDATE SET
            Name = excluded.Name,
            DockerImageId = excluded.DockerImageId,
            ImageId = excluded.ImageId,
            Created = excluded.Created,
            Updated = excluded.Updated,
            State = excluded.State,
            Stack = excluded.Stack,
            Ports = excluded.Ports,
            StackId = excluded.StackId,
            IsSystem = excluded.IsSystem,
            SystemRole = excluded.SystemRole,
            HasCitadelOwnershipLabels = excluded.HasCitadelOwnershipLabels,
            IsSwarmTask = excluded.IsSwarmTask,
            DockerNodeId = excluded.DockerNodeId,
            ProjectionObservedAt = excluded.ProjectionObservedAt,
            ProjectionStaleSince = excluded.ProjectionStaleSince,
            ProjectionStaleReason = excluded.ProjectionStaleReason;
    """;

        return db.ExecuteAsync(sql, containers.Select(c => new
        {
            Id = c.Id,
            PlatformId = c.PlatformId,
            DockerContainerId = c.DockerContainerId,
            Name = c.Name,
            DockerImageId = c.DockerImageId,
            Created = c.Created,
            Updated = c.Updated,
            State = EnumFormatter<ContainerStateStatus>.GetValue(c.State),
            Stack = c.DockerStack,
            ImageId = c.ImageId,
            StackId = c.StackId,
            IsSystem = c.IsSystem,
            SystemRole = c.SystemRole?.ToString(),
            HasCitadelOwnershipLabels = c.HasCitadelOwnershipLabels,
            IsSwarmTask = c.IsSwarmTask,
            DockerNodeId = c.DockerNodeId,
            ProjectionObservedAt = c.ProjectionObservedAt,
            ProjectionStaleSince = c.ProjectionStaleSince,
            ProjectionStaleReason = c.ProjectionStaleReason,
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
        WHERE Id = ANY(@Ids)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Ids = ids.ToArray(),
                State = EnumFormatter<ContainerStateStatus>.GetValue(state),
                Updated = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            },
            transaction: tx()
        );
    }

    public Task<int> MarkNodeProjectionStaleAsync(
        Guid platformId,
        string dockerNodeId,
        string reason,
        long staleSince,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Containers
            SET ProjectionStaleSince = COALESCE(ProjectionStaleSince, @StaleSince),
                ProjectionStaleReason = @Reason
            WHERE PlatformId = @PlatformId
              AND DockerNodeId = @DockerNodeId
            """;
        return db.ExecuteAsync(
            sql,
            new { PlatformId = platformId, DockerNodeId = dockerNodeId, Reason = reason, StaleSince = staleSince },
            tx());
    }

    public Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken)
    {
        var conditions = new List<string>
        {
            "Id = @Id"
        };

        if (checkRowVersion == true)
            conditions.Add("RowVersion = @RowVersion");

        if (state == ResourceControlState.Processing)
            conditions.Add("ControlState <> @ProcessingState");

        var sql = $"""
            UPDATE Containers
            SET
                ControlState = @State,
                ControlStartedAt = @StartedAt,
                ControlTriggeredBy = @ControlTriggeredBy,
                RowVersion = RowVersion + 1
            WHERE {string.Join(" AND ", conditions)}
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                ControlTriggeredBy = controlTriggeredBy,
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                ProcessingState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
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
        WHERE Id = ANY(@Ids)
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = ids.ToArray() },
            transaction: tx()
        );
    }
}
