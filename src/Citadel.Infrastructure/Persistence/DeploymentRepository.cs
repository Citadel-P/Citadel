using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class DeploymentRepository(IDbConnection db, Func<IDbTransaction> tx) : IDeploymentRepository
{
    private const string BaseSelect = """
    SELECT
        d.Id,
        d.Name, 
        d.Description, 
        d.PlatformId, 
        d.Status, 
        d.CreatedAt, 
        d.CreatedByActorId, 
        d.AutoUpdateState_LastCheckedAt, 
        d.AutoUpdateState_Status,
        d.AutoUpdateState_CurrentDigest,
        d.AutoUpdateState_RemoteDigest,
        d.AutoUpdateState_LastError,
        d.ControlState,
        d.ControlStartedAt,
        d.RowVersion,
        d.ControlTriggeredBy,
        c.Id AS Container_ContainerId,
        c.DockerContainerId AS Container_DockerContainerId,
        p.Name AS Platform_Name,
        p.Status AS Platform_Status,
        i.Name as Image_Name,
        i.Id AS Image_Id,
        i.DockerImageId AS Image_DockerImageId
    FROM Deployments d 
    LEFT JOIN Containers c
        ON d.Id = c.DeploymentId
    LEFT JOIN Images i 
        ON c.ImageId = i.Id
    LEFT JOIN Platforms p 
        ON d.PlatformId = p.Id
    """;

    public async Task<Deployment?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                d.*,
                c.Id AS Container_ContainerId,
                c.DockerContainerId AS Container_DockerContainerId,
                ei.Info AS ActivityEvent_ActivityEventInfo,
                ei.EventType AS ActivityEvent_EventType,
                ei.Status AS ActivityEvent_Status,
                ei.Id AS ActivityEvent_Id,
                ei.CreatedAt AS ActivityEvent_CreatedAt
                FROM Deployments d
            LEFT JOIN Containers c 
                ON c.DeploymentId = d.Id
            LEFT JOIN LATERAL (
                SELECT e.Id, e.EventType, e.Status, e.Info, e.CreatedAt
                FROM ActivityEvents e
                WHERE e.ResourceId = d.Id 
                  AND e.ResourceType = 'Deployment'
                  AND e.EventType NOT IN ('DeploymentUpdated', 'DeploymentRenamed')
                ORDER BY e.CreatedAt DESC
                LIMIT 1
            ) ei ON TRUE
            WHERE d.Id = @Id LIMIT 1
            """;
            
        var result = await db.QuerySingleOrDefaultAsync<DeploymentDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }
    
    public async Task<Deployment?> GetInfoAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = BaseSelect + " "+ "WHERE d.Id = @Id LIMIT 1";

        var result = await db.QuerySingleOrDefaultAsync<DeploymentDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        var d = result?.ToDomain();
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Deployment>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT
            d.*,
            c.Id AS Container_ContainerId,
            c.DockerContainerId AS Container_DockerContainerId,
            p.Name AS Platform_Name,
            p.status AS Platform_Status
        FROM Deployments d 
        LEFT JOIN Containers c
            ON d.Id = c.DeploymentId
        LEFT JOIN Platforms p 
            ON d.PlatformId = p.Id
            WHERE d.Id = ANY(@Ids)
            ORDER BY d.CreatedAt DESC, d.Name ASC
        """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new { Ids = ids.ToArray(), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Deployment>> GetStuckDeploymentsAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        const string sql = BaseSelect + " " + """
            WHERE 
                d.ControlState = 'Processing'
                AND d.ControlStartedAt < @ControlStartedAt
            ORDER BY 
                d.ControlStartedAt ASC
            """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new 
        {
            ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s,
            cancellationToken 
        }, transaction: tx());

        return result.ToDomain();
    }
    
    public Task<bool> ExistsAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE PlatformId = @PlatformId)";
        return db.ExecuteScalarAsync<bool>(sql, new { PlatformId = platformId, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(string name, Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE name = @Name AND PlatformId = @PlatformId)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, PlatformId = platformId, cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, string name, Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE Name=@Name AND Id != @Id AND PlatformId = @PlatformId)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id, PlatformId = platformId, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Deployments (
                Id, Name, Description, PlatformId, Status, CreatedAt, CreatedByActorId, Spec, AutoUpdateState_LastCheckedAt, AutoUpdateState_Status, AutoUpdateState_CurrentDigest,
                AutoUpdateState_RemoteDigest, AutoUpdateState_LastError
            ) VALUES (
                 @Id, @Name, @Description, @PlatformId, @Status, @CreatedAt, @CreatedByActorId, @Spec::json,
                 @AutoUpdateState_LastCheckedAt, @AutoUpdateState_Status, @AutoUpdateState_CurrentDigest,
                 @AutoUpdateState_RemoteDigest, @AutoUpdateState_LastError
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id,
            Name = deployment.Name,
            Description = deployment.Description,
            PlatformId = deployment.PlatformId,
            CreatedByActorId = deployment.CreatedByActorId,
            CreatedAt = deployment.CreatedAt,
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status),
            Spec = JsonSerializer.Serialize(deployment.Spec, DeploymentJsonContext.Default.DeploymentSpec),
            AutoUpdateState_LastCheckedAt = deployment.AutoUpdateState?.LastCheckedAt,
            AutoUpdateState_Status = deployment.AutoUpdateState != null ? EnumFormatter<AutoUpdateStatus>.GetValue(deployment.AutoUpdateState.Status) : null,
            AutoUpdateState_CurrentDigest = deployment.AutoUpdateState?.CurrentDigest,
            AutoUpdateState_RemoteDigest = deployment.AutoUpdateState?.RemoteDigest,
            AutoUpdateState_LastError = deployment.AutoUpdateState?.LastError,

        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetImageLookupAsync(Guid deploymentId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT i.Id, i.Name
            FROM Deployments d
            JOIN Images i ON i.PlatformId = d.PlatformId
            WHERE d.Id = @DeploymentId
            ORDER BY i.Name
         """;

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            DeploymentId = deploymentId,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<IEnumerable<Deployment>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                d.*,
                c.Id AS Container_ContainerId,
                c.DockerImageId AS Container_DockerImageId
            FROM Deployments d
            LEFT JOIN Containers c
                ON c.DeploymentId = d.Id
            """;
        var result = await db.QueryAsync<DeploymentDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Deployment>> GetInfoAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                d.Id,
                d.Name, 
                d.Description, 
                d.PlatformId, 
                d.RowVersion,
                d.ControlState,
                d.ControlStartedAt,
                d.ControlTriggeredBy,
                d.Status,
                d.CreatedAt, 
                d.CreatedByActorId, 
                d.AutoUpdateState_LastCheckedAt, 
                d.AutoUpdateState_Status,
                d.AutoUpdateState_CurrentDigest,
                d.AutoUpdateState_RemoteDigest,
                d.AutoUpdateState_LastError,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status,
                i.Name as Image_Name,
                i.Id AS Image_Id,
                i.DockerImageId AS Image_DockerImageId
            FROM Deployments d 
            LEFT JOIN Platforms p 
                ON d.PlatformId = p.Id
            LEFT JOIN Containers c 
                ON d.Id = c.DeploymentId
            LEFT JOIN Images i 
                ON c.ImageId = i.Id
            ORDER BY 
                d.CreatedAt DESC,
                d.Name ASC
            """;
        var result = await db.QueryAsync<DeploymentDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<PlatformConnectionInfo?> GetPlatformByDeploymentIdAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                Platforms.Id,
                Platforms.Name,
                Platforms.Address,
                Platforms.ConnectorType
            FROM Deployments d
            JOIN Platforms p ON p.Id = d.PlatformId
            WHERE d.Id = @DeploymentId
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformConnectionInfoDto>(sql, new { DeploymentId = id }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<string?> GetContainerIdAsync(Guid deploymentId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT c.DockerContainerId FROM Deployments d JOIN Containers c ON c.DeploymentId = d.Id WHERE d.Id = @DeploymentId LIMIT 1";
        return db.QuerySingleOrDefaultAsync<string>(sql, new { DeploymentId = deploymentId, cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<Deployment>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + BaseSelect + " WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "d.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " ORDER BY d.CreatedAt DESC, d.Name ASC";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var result = await db.QueryAsync<DeploymentDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<bool> CanAccessAsync(Guid userId, Guid deploymentId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT EXISTS (SELECT 1 FROM Deployments d WHERE d.Id = @DeploymentId AND 
            {{AuthorizationSql.ResourcePredicatePrefix}}d.Id{{AuthorizationSql.ResourcePredicateSuffix}})
        """;

        return db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId,
            DeploymentId = deploymentId,
            ResourceType = (int)ResourceType.Deployment,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
            SpecificPermission = (int)SpecificPermission.None,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT p.Id, p.Name
                FROM Platforms p
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT p.Id, p.Name
                FROM Deployments d
                JOIN Platforms p ON p.Id = d.PlatformId
                WHERE d.Id = @DeploymentId
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Platform,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            DeploymentId = deploymentId,
            cancellationToken
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT r.Id, r.Name
                FROM Registries r
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}r.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT r.Id, r.Name
                FROM Deployments d
                JOIN Registries r ON r.Id = CAST(d.Spec -> 'Image' ->> 'RegistryId' AS uuid)
                WHERE d.Id = @DeploymentId
                  AND d.Spec -> 'Image' ->> '$type' = 'External'
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Registry,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            DeploymentId = deploymentId,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<IEnumerable<Deployment>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = BaseSelect + " " + """
            WHERE d.PlatformId = @PlatformId
            ORDER BY d.CreatedAt DESC, d.Name ASC
        """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new { PlatformId = platformId, cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Deployments
            SET Name = @Name, Description = @Description, PlatformId = @PlatformId, Status = @Status, Spec = @Spec::json,
                AutoUpdateState_LastCheckedAt = @AutoUpdateState_LastCheckedAt, AutoUpdateState_Status = @AutoUpdateState_Status, 
                AutoUpdateState_CurrentDigest = @AutoUpdateState_CurrentDigest, AutoUpdateState_RemoteDigest = @AutoUpdateState_RemoteDigest, 
                AutoUpdateState_LastError = @AutoUpdateState_LastError
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id,
            Name = deployment.Name,
            Description = deployment.Description,
            PlatformId = deployment.PlatformId,
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status),
            Spec = JsonSerializer.Serialize(deployment.Spec, DeploymentJsonContext.Default.DeploymentSpec),
            AutoUpdateState_LastCheckedAt = deployment.AutoUpdateState?.LastCheckedAt,
            AutoUpdateState_Status = deployment.AutoUpdateState != null ? EnumFormatter<AutoUpdateStatus>.GetValue(deployment.AutoUpdateState.Status) : null,
            AutoUpdateState_CurrentDigest = deployment.AutoUpdateState?.CurrentDigest,
            AutoUpdateState_RemoteDigest = deployment.AutoUpdateState?.RemoteDigest,
            AutoUpdateState_LastError = deployment.AutoUpdateState?.LastError,
        }, transaction: tx());
    }

    public Task<int> UpdateProcessingAsync(Guid id, DeploymentStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken)
    {
        var conditions = new List<string>
        {
            "Id = @Id"
        };

        if (checkRowVersion == true)
            conditions.Add("RowVersion = @RowVersion");

        var sql = $"""
            UPDATE Deployments
            SET
                ControlState = @State,
                ControlStartedAt = @StartedAt,
                ControlTriggeredBy = @ControlTriggeredBy,
                Status = @Status,
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
                Status = EnumFormatter<DeploymentStatus>.GetValue(status),
                RowVersion = rowVersion,
                StartedAt = startedAt
            },
            transaction: tx()
        );
    }

    public async Task<int> UpdateStatusAsync(IEnumerable<Guid> ids, DeploymentStatus status, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Deployments
            SET Status = @Status
            WHERE Id = ANY(@Ids)
        """;
        return await db.ExecuteAsync(
            sql,
            new
            {
                Ids = ids.ToArray(),
                Status = EnumFormatter<DeploymentStatus>.GetValue(status)
            },
            transaction: tx()
        );
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Deployments
        WHERE Id = ANY(@Ids)
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = ids.ToArray() },
            transaction: tx()
        );
    }
}
