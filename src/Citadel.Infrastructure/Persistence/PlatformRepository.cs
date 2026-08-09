using System.Data;
using System.Text.Json;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class PlatformRepository(IDbConnection db, Func<IDbTransaction> tx) : IPlatformRepository 
{
    private const string WorkloadStatusColumns = """
        deployment_counts.DeploymentCount,
        deployment_counts.DeploymentHealthyCount,
        deployment_counts.DeploymentDegradedCount,
        deployment_counts.DeploymentFailedCount,
        deployment_counts.DeploymentStoppedCount,
        deployment_counts.DeploymentInProgressCount,
        deployment_counts.DeploymentUnknownCount,
        stack_counts.StackCount,
        stack_counts.StackHealthyCount,
        stack_counts.StackDegradedCount,
        stack_counts.StackFailedCount,
        stack_counts.StackStoppedCount,
        stack_counts.StackPausedCount,
        stack_counts.StackInProgressCount,
        stack_counts.StackUnknownCount
        """;

    private const string WorkloadStatusJoins = """
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) AS DeploymentCount,
                COUNT(*) FILTER (WHERE d.Status = @DeploymentHealthyStatus) AS DeploymentHealthyCount,
                COUNT(*) FILTER (WHERE d.Status = @DeploymentDegradedStatus) AS DeploymentDegradedCount,
                COUNT(*) FILTER (WHERE d.Status = @DeploymentFailedStatus) AS DeploymentFailedCount,
                COUNT(*) FILTER (WHERE d.Status = @DeploymentStoppedStatus) AS DeploymentStoppedCount,
                COUNT(*) FILTER (WHERE d.Status = ANY(@DeploymentInProgressStatuses)) AS DeploymentInProgressCount,
                COUNT(*) FILTER (WHERE d.Status = @DeploymentUnknownStatus) AS DeploymentUnknownCount
            FROM Deployments d
            WHERE d.PlatformId = p.Id
        ) deployment_counts ON TRUE
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) AS StackCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackHealthyStatus) AS StackHealthyCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackDegradedStatus) AS StackDegradedCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackFailedStatus) AS StackFailedCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackStoppedStatus) AS StackStoppedCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackPausedStatus) AS StackPausedCount,
                COUNT(*) FILTER (WHERE sr.Status = ANY(@StackInProgressStatuses)) AS StackInProgressCount,
                COUNT(*) FILTER (WHERE sr.Status = @StackUnknownStatus) AS StackUnknownCount
            FROM Stacks st
            JOIN StackReleases sr ON sr.Id = st.CurrentStackReleaseId
            WHERE sr.PlatformId = p.Id
        ) stack_counts ON TRUE
        """;

    public async Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Id = @PlatformId";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(sql, new { PlatformId = platformId }, transaction: tx());

        return result?.ToDomain();
    }

    public async Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Platforms WHERE Name = @Name";
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(sql, new { Name = name }, transaction: tx());

        return result?.ToDomain();
    }

    public async Task<Platform?> GetByDaemonIdAsync(
        string daemonId,
        Guid? excludePlatformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT p.*
            FROM Platforms p
            WHERE BTRIM(p.PlatformDescriptor->>'DaemonId') = @DaemonId
              AND (@ExcludePlatformId IS NULL OR p.Id <> @ExcludePlatformId)
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(
            sql,
            new
            {
                DaemonId = daemonId,
                ExcludePlatformId = excludePlatformId
            },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<Platform?> GetByClusterIdAsync(
        string clusterId,
        Guid? excludePlatformId,
        CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT p.*
            FROM Platforms p
            WHERE p.ClusterId = @ClusterId
              AND (@ExcludePlatformId IS NULL OR p.Id <> @ExcludePlatformId)
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<PlatformDto>(
            sql,
            new
            {
                ClusterId = clusterId,
                ExcludePlatformId = excludePlatformId
            },
            transaction: tx());

        return result?.ToDomain();
    }

    public Task<int?> PlatformNameExistsAsync(string name, Guid excludePlatformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 1
            FROM Platforms
            WHERE Name = @Name AND Id != @Id
            LIMIT 1
        """;
        return db.ExecuteScalarAsync<int?>(sql, new { Id = excludePlatformId, Name = name }, transaction: tx());
    }

    public Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS(SELECT 1 FROM Platforms WHERE Address=@Address OR Name=@Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Address = address }, transaction: tx());
    }

    public async Task<PlatformConnectionInfo?> GetInfoAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, Name, Address, ConnectorType
            FROM Platforms
            WHERE Id = @PlatformId
            LIMIT 1
        """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformConnectionInfoDto>(sql, new { PlatformId = platformId }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT Id, Address, ConnectorType FROM Platforms ORDER BY Name";
        var result = await db.QueryAsync<PlatformConnectionInfoDto>(sql, transaction: tx());
        return result.Select(s => s.ToDomain());
    }

    public Task<bool> CanAccessAsync(Guid userId, Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}} SELECT EXISTS (SELECT 1 FROM Platforms p WHERE p.Id = @PlatformId AND 
            {{AuthorizationSql.ResourcePredicatePrefix}}p.Id{{AuthorizationSql.ResourcePredicateSuffix}})
        """;

        return db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId,
            PlatformId = platformId,
            ResourceType = (int)ResourceType.Platform,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
            SpecificPermission = (int)SpecificPermission.None
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetDeploymentLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT d.Id, d.Name
                FROM Deployments d
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}d.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT d.Id, d.Name
                FROM Deployments d
                WHERE d.PlatformId = @PlatformId
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Deployment,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            PlatformId = platformId
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetStackLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT lookup.Id, lookup.Name
            FROM (
                SELECT s.Id, s.Name
                FROM Stacks s
                WHERE {{AuthorizationSql.ResourcePredicatePrefix}}s.Id{{AuthorizationSql.ResourcePredicateSuffix}}

                UNION

                SELECT s.Id, s.Name
                FROM Stacks s
                JOIN StackReleases sr ON sr.Id = s.CurrentStackReleaseId
                WHERE sr.PlatformId = @PlatformId
            ) lookup
            ORDER BY lookup.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read);

        return db.QueryAsync<ResourceInfo>(sql, new
        {
            UserId = userId,
            ResourceType = (int)ResourceType.Stack,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)SpecificPermission.None,
            PlatformId = platformId
        }, transaction: tx());
    }

    public Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken)
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
                FROM Images i
                JOIN Registries r ON r.Id = i.RegistryId
                WHERE i.PlatformId = @PlatformId
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
            PlatformId = platformId
        }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS(SELECT 1 FROM Platforms WHERE Id = @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Id = id }, transaction: tx());
    }

    public async Task<int> AddAsync(Platform platform, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null)
    {
        string sql = ResourceTagSql.InputTagsCte + """
            inserted_platform AS (
                INSERT INTO Platforms (
                    Id, Name, Address, Description, NetworkCount, VolumeCount, ImageCount, CpuCount, MemTotal, ServerVersion, AgentVersion, Status, ConnectorType, PlatformDescriptor, ClusterId, PruneHistoricalSwarmTaskContainers)
                SELECT
                    @Id, @Name, @Address, @Description, @NetworkCount, @VolumeCount, @ImageCount, @CpuCount, @MemTotal, @ServerVersion, @AgentVersion, @Status, @ConnectorType, @PlatformDescriptor::json, @ClusterId, @PruneHistoricalSwarmTaskContainers
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                ON CONFLICT (ClusterId) WHERE ClusterId IS NOT NULL DO NOTHING
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_platform", "p") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_platform", "inserted_platform", "inserted_tags");
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(sql, new
        {
            Id = platform.Id,
            Name = platform.Name,
            Address = platform.Address,
            Description = platform.Description,
            NetworkCount = platform.NetworkCount,
            VolumeCount = platform.VolumeCount,
            ImageCount = platform.ImageCount,
            CpuCount = platform.CpuCount,
            MemTotal = platform.MemTotal,
            ServerVersion = platform.ServerVersion,
            AgentVersion = platform.AgentVersion,
            Status = EnumFormatter<PlatformStatus>.GetValue(platform.Status),
            ConnectorType = EnumFormatter<PlatformConnectorType>.GetValue(platform.ConnectorType),
            PlatformDescriptor = JsonSerializer.Serialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor),
            platform.ClusterId,
            platform.PruneHistoricalSwarmTaskContainers,
            CreatedAt = DateTime.UtcNow,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Platform),
            TagIds = tagIdArray,
            TagCreatedByActorId = tagCreatedByActorId ?? Constants.SystemId
        }, transaction: tx());

        platform.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM Platforms WHERE Id = @PlatformId";
        return db.ExecuteAsync(sql, new { PlatformId = platformId }, transaction: tx());
    }

    public Task<int> UpdateAsync(Platform platform, CancellationToken cancellationToken)
    {
        const string updateSql = """
            UPDATE Platforms SET
                Name = @Name,
                Address = @Address,
                Description = @Description,
                NetworkCount = @NetworkCount,
                VolumeCount = @VolumeCount,
                ImageCount = @ImageCount,
                CpuCount = @CpuCount,
                MemTotal = @MemTotal,
                ServerVersion = @ServerVersion,
                AgentVersion = @AgentVersion,
                PlatformDescriptor = @PlatformDescriptor::json,
                ClusterId = @ClusterId,
                PruneHistoricalSwarmTaskContainers = @PruneHistoricalSwarmTaskContainers,
                Status = @Status
            WHERE Id = @Id
         """;

        return db.ExecuteAsync(updateSql, new
        {
            platform.Name,
            platform.Address,
            platform.Description,
            platform.NetworkCount,
            platform.VolumeCount,
            platform.ImageCount,
            platform.CpuCount,
            platform.MemTotal,
            platform.ServerVersion,
            platform.AgentVersion,
            PlatformDescriptor = JsonSerializer.Serialize(platform.PlatformDescriptor, PlatformJsonContext.Default.PlatformDescriptor),
            platform.ClusterId,
            platform.PruneHistoricalSwarmTaskContainers,
            Status = EnumFormatter<PlatformStatus>.GetValue(platform.Status),
            Id = platform.Id
        }, transaction: tx());
    }

    public async Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            SELECT p.*,
                {{WorkloadStatusColumns}},
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes,
                s.DiskUsedBytes as Stat_DiskUsedBytes,
                s.DiskTotalBytes as Stat_DiskTotalBytes,
                s.DiskUsage as Stat_DiskUsage,
                {{ResourceTagSql.PlatformTagAggregate}}
            FROM Platforms p
            {{WorkloadStatusJoins}}
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            WHERE p.Id = @Id
            LIMIT 1;
        """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformWithSingleStatDto>(sql, new PlatformWorkloadQueryParameters
        {
            Id = platformId,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Platform)
        }, tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<Platform>> GetPlatformsWithLatestStatByIdsAsync(IReadOnlyCollection<Guid> platformIds, CancellationToken cancellationToken)
    {
        if (platformIds.Count == 0)
        {
            return [];
        }

        const string sql = $$"""
            SELECT p.*,
                {{WorkloadStatusColumns}},
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes,
                s.DiskUsedBytes as Stat_DiskUsedBytes,
                s.DiskTotalBytes as Stat_DiskTotalBytes,
                s.DiskUsage as Stat_DiskUsage,
                {{ResourceTagSql.PlatformTagAggregate}}
            FROM Platforms p
            {{WorkloadStatusJoins}}
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            WHERE p.Id = ANY(@Ids)
            ORDER BY p.Name;
        """;

        var idArray = platformIds as Guid[] ?? [.. platformIds];
        var result = await db.QueryAsync<PlatformWithSingleStatDto>(sql, new PlatformWorkloadQueryParameters
        {
            Ids = idArray,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Platform)
        }, tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        const string sql = $$"""
            SELECT p.*,
                {{WorkloadStatusColumns}},
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes,
                s.DiskUsedBytes as Stat_DiskUsedBytes,
                s.DiskTotalBytes as Stat_DiskTotalBytes,
                s.DiskUsage as Stat_DiskUsage,
                {{ResourceTagSql.PlatformTagAggregate}}
            FROM Platforms p
            {{WorkloadStatusJoins}}
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            WHERE {{ResourceTagSql.PlatformTagFilterPredicate}}
            ORDER BY p.Name;
        """;

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<PlatformWithSingleStatDto>(sql, new PlatformWorkloadQueryParameters
        {
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Platform),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<Platform>> GetAuthorizedWithLatestStatAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            
            SELECT p.*,
                {{WorkloadStatusColumns}},
                s.Id as Stat_Id,
                s.Created as Stat_Created,
                s.CpuUsage as Stat_CpuUsage,
                s.MemoryUsage as Stat_MemoryUsage,
                s.RxBytes as Stat_RxBytes,
                s.TxBytes as Stat_TxBytes,
                s.DiskUsedBytes as Stat_DiskUsedBytes,
                s.DiskTotalBytes as Stat_DiskTotalBytes,
                s.DiskUsage as Stat_DiskUsage,
                {{ResourceTagSql.PlatformTagAggregate}}
            FROM Platforms p
            {{WorkloadStatusJoins}}
            LEFT JOIN PlatformStats s ON s.Id = (
                SELECT Id FROM PlatformStats
                WHERE PlatformId = p.Id
                ORDER BY Created DESC
                LIMIT 1
            )
            WHERE {{AuthorizationSql.ResourcePredicatePrefix}}p.Id {{AuthorizationSql.ResourcePredicateSuffix}}
              AND {{ResourceTagSql.PlatformTagFilterPredicate}}
            ORDER BY p.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<PlatformWithSingleStatDto>(sql, new PlatformWorkloadQueryParameters
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.Platform),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());

        return result.ToDomain();
    }

    internal sealed class PlatformWorkloadQueryParameters
    {
        public Guid Id { get; init; }
        public Guid[] Ids { get; init; } = [];
        public Guid UserId { get; init; }
        public int ResourceType { get; init; }
        public int GrantedPermissionMask { get; init; }
        public int SpecificPermission { get; init; }
        public string TagResourceType { get; init; } = string.Empty;
        public Guid[] TagIds { get; init; } = [];
        public int TagIdsLength { get; init; }
        public string DeploymentHealthyStatus { get; } =
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Healthy);
        public string DeploymentDegradedStatus { get; } =
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Degraded);
        public string DeploymentFailedStatus { get; } =
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Failed);
        public string DeploymentStoppedStatus { get; } =
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Stopped);
        public string DeploymentUnknownStatus { get; } =
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Unknown);
        public string[] DeploymentInProgressStatuses { get; } =
        {
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Created),
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Pending),
            EnumFormatter<DeploymentStatus>.GetValue(DeploymentStatus.Applying)
        };
        public string StackHealthyStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Healthy);
        public string StackDegradedStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Degraded);
        public string StackFailedStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Failed);
        public string StackStoppedStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Stopped);
        public string StackPausedStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Paused);
        public string StackUnknownStatus { get; } =
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Unknown);
        public string[] StackInProgressStatuses { get; } =
        {
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Created),
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Pending),
            EnumFormatter<StackReleaseStatus>.GetValue(StackReleaseStatus.Applying)
        };
    }

    public async Task<IEnumerable<Platform>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT p.*
            FROM Platforms p
            WHERE {{AuthorizationSql.ResourcePredicatePrefix}}p.Id {{AuthorizationSql.ResourcePredicateSuffix}}
            ORDER BY p.Name
         """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var result = await db.QueryAsync<PlatformWithSingleStatDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission
        }, transaction: tx());

        return result.ToDomain();
    }

    public async Task<PlatformConnectionInfo?> GetPlatformByContainerIdAsync(string dockerContainerId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                Platforms.Id,
                Platforms.Name,
                Platforms.Address,
                Platforms.ConnectorType
            FROM Containers
            JOIN Platforms ON Platforms.Id = Containers.PlatformId
            WHERE Containers.DockerContainerId LIKE @ContainerIdPrefix || '%'
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<PlatformConnectionInfoDto>(sql, new { ContainerIdPrefix = dockerContainerId }, transaction: tx());
        return result?.ToDomain();
    }
}
