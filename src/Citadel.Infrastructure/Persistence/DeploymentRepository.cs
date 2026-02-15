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
        p.Status AS Platform_Status
    FROM Deployments d 
    LEFT JOIN Containers c
        ON d.Id = c.DeploymentId
    LEFT JOIN Platforms p 
        ON d.PlatformId = p.Id
    """;

    public async Task<Deployment?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                d.*,
                c.Id AS Container_ContainerId,
                c.DockerContainerId AS Container_DockerContainerId
                FROM Deployments d
            LEFT JOIN Containers c ON c.DeploymentId = d.Id
            WHERE d.Id = @Id LIMIT 1
            """;
            
        var result = await db.QuerySingleOrDefaultAsync<DeploymentDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }
    
    public async Task<Deployment?> GetInfoAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = BaseSelect + " "+ "WHERE d.Id = @Id LIMIT 1";

        var result = await db.QuerySingleOrDefaultAsync<DeploymentDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
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
            WHERE d.Id IN (SELECT value FROM json_each(@Ids))
            ORDER BY d.CreatedAt DESC, d.Name ASC
        """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Deployment>> GetStuckDeploymentsAsync(int timeout_s = 60, CancellationToken cancellationToken = default)
    {
        const string sql = BaseSelect + " " +"""
            WHERE 
                d.ControlState = @ControlState
                AND d.ControlStartedAt < @ControlStartedAt
            ORDER BY 
                d.ControlStartedAt ASC
            """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new 
        {
            ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
            ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - timeout_s,
            cancellationToken 
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<bool> ExistsAsync(string name, Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE name = @Name AND PlatformId = @PlatformId)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, PlatformId = platformId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, string name, Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE Name=@Name AND Id != @Id AND PlatformId = @PlatformId)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id.Format(), PlatformId = platformId.Format(), cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Deployments (
                Id, Name, Description, PlatformId, Status, CreatedAt, CreatedByActorId, Spec, AutoUpdateState_LastCheckedAt, AutoUpdateState_Status, AutoUpdateState_CurrentDigest,
                AutoUpdateState_RemoteDigest, AutoUpdateState_LastError
            ) VALUES (
                 @Id, @Name, @Description, @PlatformId, @Status, @CreatedAt, @CreatedByActorId, @Spec,
                 @AutoUpdateState_LastCheckedAt, @AutoUpdateState_Status, @AutoUpdateState_CurrentDigest,
                 @AutoUpdateState_RemoteDigest, @AutoUpdateState_LastError
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id.Format(),
            Name = deployment.Name,
            Description = deployment.Description,
            PlatformId = deployment.PlatformId.Format(),
            CreatedByActorId = deployment.CreatedByActorId.Format(),
            CreatedAt = deployment.CreatedAt.ToString(),
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status),
            Spec = JsonSerializer.Serialize(deployment.Spec, DeploymentJsonContext.Default.DeploymentSpec),
            AutoUpdateState_LastCheckedAt = deployment.AutoUpdateState?.LastCheckedAt.ToString(),
            AutoUpdateState_Status = deployment.AutoUpdateState != null ? EnumFormatter<AutoUpdateStatus>.GetValue(deployment.AutoUpdateState.Status) : null,
            AutoUpdateState_CurrentDigest = deployment.AutoUpdateState?.CurrentDigest,
            AutoUpdateState_RemoteDigest = deployment.AutoUpdateState?.RemoteDigest,
            AutoUpdateState_LastError = deployment.AutoUpdateState?.LastError,

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
                i.Id AS Image_Id
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

    public async Task<IEnumerable<Deployment>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = BaseSelect + " " + """
            WHERE d.PlatformId = @PlatformId
            ORDER BY d.CreatedAt DESC, d.Name ASC
        """;
        var result = await db.QueryAsync<DeploymentDto>(sql, new { PlatformId = platformId.Format(), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE Deployments
            SET Name = @Name, Description = @Description, PlatformId = @PlatformId, Status = @Status, Spec = @Spec,
                AutoUpdateState_LastCheckedAt = @AutoUpdateState_LastCheckedAt, AutoUpdateState_Status = @AutoUpdateState_Status, 
                AutoUpdateState_CurrentDigest = @AutoUpdateState_CurrentDigest, AutoUpdateState_RemoteDigest = @AutoUpdateState_RemoteDigest, 
                AutoUpdateState_LastError = @AutoUpdateState_LastError
            WHERE Id = @Id
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id.Format(),
            Name = deployment.Name,
            Description = deployment.Description,
            PlatformId = deployment.PlatformId.Format(),
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status),
            Spec = JsonSerializer.Serialize(deployment.Spec, DeploymentJsonContext.Default.DeploymentSpec),
            AutoUpdateState_LastCheckedAt = deployment.AutoUpdateState?.LastCheckedAt.ToString(),
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
                Id = id.Format(),
                ControlTriggeredBy = controlTriggeredBy?.Format(),
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
            WHERE Id IN (
                SELECT value FROM json_each(@Ids)
            )
        """;
        return await db.ExecuteAsync(
            sql,
            new
            {
                Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid),
                Status = EnumFormatter<DeploymentStatus>.GetValue(status)
            },
            transaction: tx()
        );
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        DELETE FROM Deployments
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
