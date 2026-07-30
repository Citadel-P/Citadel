using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common.Models;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Hosting.Common;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class ActivityEventRepository(IDbConnection db, Func<IDbTransaction> tx) : IActivityEventRepository
{
    public Task<int> AddAsync(ActivityEvent activityEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO ActivityEvents (
            Id, PlatformId, ResourceId, ResourceName, ResourceType, EventType, Info, CreatedByActorId, Status, CreatedAt
        )
        VALUES (
            @Id, @PlatformId, @ResourceId, @ResourceName, @ResourceType, @EventType, @Info::json, @CreatedByActorId, @Status, @CreatedAt
        )";
        return db.ExecuteAsync(sql, new
        {
            Id = activityEvent.Id,
            PlatformId = activityEvent.PlatformId != null ? activityEvent.PlatformId : null,
            ResourceId = activityEvent.ResourceId != null ? activityEvent.ResourceId : null,
            ResourceName = activityEvent.ResourceName,
            EventType = EnumFormatter<ActivityEventType>.GetValue(activityEvent.EventType),
            ResourceType = EnumFormatter<ActivityResourceType>.GetValue(activityEvent.ResourceType),
            Status = EnumFormatter<ActivityStatus>.GetValue(activityEvent.Status),
            Info = JsonSerializer.Serialize(activityEvent.Info, EventInfoJsonContext.Default.ActivityEventInfo),
            CreatedByActorId = activityEvent.CreatedByActorId,
            CreatedAt = activityEvent.CreatedAt
        },
        transaction: tx());
    }

    public async Task<ActivityEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken)
    {
        var sql = """
            SELECT 
                a.*,
                u.Name AS Actor_Name,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status,
                ac.Type AS Actor_Type
            FROM ActivityEvents a
            LEFT JOIN Users u ON a.CreatedByActorId = u.ActorId
            LEFT JOIN Actors ac ON a.CreatedByActorId = ac.Id
            LEFT JOIN Platforms p ON a.PlatformId = p.Id
            WHERE a.Id = @Id
            LIMIT 1;
        """;
        var result = await db.QuerySingleOrDefaultAsync<ActivityEventDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<ActivityEvent?> GetAuthorizedByIdAsync(
        Guid id,
        Guid userId,
        CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.AuthorizedResourcesCte + """
            SELECT 
                a.*,
                u.Name AS Actor_Name,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status,
                ac.Type AS Actor_Type
            FROM ActivityEvents a
            LEFT JOIN Users u ON a.CreatedByActorId = u.ActorId
            LEFT JOIN Actors ac ON a.CreatedByActorId = ac.Id
            LEFT JOIN Platforms p ON a.PlatformId = p.Id
            WHERE a.Id = @Id
              AND EXISTS (
                  SELECT 1
                  FROM AuthorizedResources authorized
                  WHERE authorized.ResourceType = CASE a.ResourceType
                      WHEN 'Platform' THEN @PlatformResourceType
                      WHEN 'Registry' THEN @RegistryResourceType
                      WHEN 'Deployment' THEN @DeploymentResourceType
                      WHEN 'Stack' THEN @StackResourceType
                      WHEN 'AlertRule' THEN @AlertResourceType
                      WHEN 'GitRepository' THEN @GitRepositoryResourceType
                      WHEN 'AutomationAction' THEN @AutomationActionResourceType
                      WHEN 'Build' THEN @BuildResourceType
                      WHEN 'BuildAgentPool' THEN @BuildAgentPoolResourceType
                      WHEN 'Volume' THEN @PlatformResourceType
                      WHEN 'BackupPolicy' THEN @BackupPolicyResourceType
                  END
                    AND (authorized.ResourceId IS NULL OR authorized.ResourceId = a.ResourceId)
              )
            LIMIT 1;
        """;

        var result = await db.QuerySingleOrDefaultAsync<ActivityEventDto>(
            sql,
            new
            {
                UserId = userId,
                Id = id,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
                PlatformResourceType = (int)ResourceType.Platform,
                RegistryResourceType = (int)ResourceType.Registry,
                DeploymentResourceType = (int)ResourceType.Deployment,
                StackResourceType = (int)ResourceType.Stack,
                AlertResourceType = (int)ResourceType.Alert,
                GitRepositoryResourceType = (int)ResourceType.GitRepository,
                AutomationActionResourceType = (int)ResourceType.AutomationAction,
                BuildResourceType = (int)ResourceType.Build,
                BuildAgentPoolResourceType = (int)ResourceType.BuildAgentPool,
                BackupPolicyResourceType = (int)ResourceType.BackupPolicy
            },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<PagedResult<ActivityEvent>> GetPagedAsync(
        Guid? resourceId,
        ActivityResourceType? resourceType,
        ActivityEventType? eventType,
        int page,
        int pageSize,
        CancellationToken cancellationToken)
    {
        const string SelectActivities = """
        SELECT 
            a.Id, 
            a.PlatformId, 
            a.ResourceId, 
            a.ResourceName, 
            a.ResourceType, 
            a.EventType, 
            a.CreatedByActorId, 
            a.CreatedAt,
            a.Status,
            a.Info,
            p.Name AS Platform_Name,
            p.Status AS Platform_Status,
            u.Name AS Actor_Name,
            ac.Type AS Actor_Type
        FROM ActivityEvents a
        LEFT JOIN Actors ac ON a.CreatedByActorId = ac.Id
        LEFT JOIN Users u ON a.CreatedByActorId = u.ActorId
        LEFT JOIN Platforms p ON a.PlatformId = p.Id
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@EventType IS NULL OR a.EventType = @EventType)
        ORDER BY a.CreatedAt DESC
        LIMIT @PageSize OFFSET @Offset;
        """;

        const string CountActivities = """
        SELECT COUNT(*)
        FROM ActivityEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@EventType IS NULL OR a.EventType = @EventType);
        """;

        var offset = (page - 1) * pageSize;

        var p = new
        {
            ResourceId = resourceId,
            ResourceType = resourceType is null ? null : EnumFormatter<ActivityResourceType>.GetValue(resourceType.Value),
            EventType = eventType is null ? null : EnumFormatter<ActivityEventType>.GetValue(eventType.Value),
            PageSize = pageSize,
            Offset = offset,
        };

        var totalCount = await db.QuerySingleAsync<int>(
            CountActivities, p, transaction: tx());

        var rows = await db.QueryAsync<ActivityEventDto>(
            SelectActivities, p, transaction: tx());

        return new PagedResult<ActivityEvent>(
            [.. rows.Select(ActivityEventMappers.ToDomain)],
            totalCount,
            page,
            pageSize);
    }

    public async Task<PagedResult<ActivityEvent>> GetAuthorizedPagedAsync(
        Guid userId,
        Guid? resourceId,
        ActivityResourceType? resourceType,
        ActivityEventType? eventType,
        int page,
        int pageSize,
        CancellationToken cancellationToken)
    {
        const string authorizationPredicate = """
            EXISTS (
                SELECT 1
                FROM AuthorizedResources authorized
                WHERE authorized.ResourceType = CASE a.ResourceType
                    WHEN 'Platform' THEN @PlatformResourceType
                    WHEN 'Registry' THEN @RegistryResourceType
                    WHEN 'Deployment' THEN @DeploymentResourceType
                    WHEN 'Stack' THEN @StackResourceType
                    WHEN 'AlertRule' THEN @AlertResourceType
                    WHEN 'GitRepository' THEN @GitRepositoryResourceType
                    WHEN 'AutomationAction' THEN @AutomationActionResourceType
                    WHEN 'Build' THEN @BuildResourceType
                    WHEN 'BuildAgentPool' THEN @BuildAgentPoolResourceType
                    WHEN 'Volume' THEN @PlatformResourceType
                    WHEN 'BackupPolicy' THEN @BackupPolicyResourceType
                END
                  AND (authorized.ResourceId IS NULL OR authorized.ResourceId = a.ResourceId)
            )
        """;

        const string selectActivities = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.AuthorizedResourcesCte + """
            SELECT 
                a.Id, 
                a.PlatformId, 
                a.ResourceId, 
                a.ResourceName, 
                a.ResourceType, 
                a.EventType, 
                a.CreatedByActorId, 
                a.CreatedAt,
                a.Status,
                a.Info,
                p.Name AS Platform_Name,
                p.Status AS Platform_Status,
                u.Name AS Actor_Name,
                ac.Type AS Actor_Type
            FROM ActivityEvents a
            LEFT JOIN Actors ac ON a.CreatedByActorId = ac.Id
            LEFT JOIN Users u ON a.CreatedByActorId = u.ActorId
            LEFT JOIN Platforms p ON a.PlatformId = p.Id
            WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
                AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
                AND (@EventType IS NULL OR a.EventType = @EventType)
                AND
            """ + authorizationPredicate + """
            ORDER BY a.CreatedAt DESC
            LIMIT @PageSize OFFSET @Offset;
        """;

        const string countActivities = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.AuthorizedResourcesCte + """
            SELECT COUNT(*)
            FROM ActivityEvents a
            WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
                AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
                AND (@EventType IS NULL OR a.EventType = @EventType)
                AND
            """ + authorizationPredicate + ";";

        var offset = (page - 1) * pageSize;
        var parameters = new
        {
            UserId = userId,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
            PlatformResourceType = (int)ResourceType.Platform,
            RegistryResourceType = (int)ResourceType.Registry,
            DeploymentResourceType = (int)ResourceType.Deployment,
            StackResourceType = (int)ResourceType.Stack,
            AlertResourceType = (int)ResourceType.Alert,
            GitRepositoryResourceType = (int)ResourceType.GitRepository,
            AutomationActionResourceType = (int)ResourceType.AutomationAction,
            BuildResourceType = (int)ResourceType.Build,
            BuildAgentPoolResourceType = (int)ResourceType.BuildAgentPool,
            BackupPolicyResourceType = (int)ResourceType.BackupPolicy,
            ResourceId = resourceId,
            ResourceType = resourceType is null ? null : EnumFormatter<ActivityResourceType>.GetValue(resourceType.Value),
            EventType = eventType is null ? null : EnumFormatter<ActivityEventType>.GetValue(eventType.Value),
            PageSize = pageSize,
            Offset = offset
        };

        var totalCount = await db.QuerySingleAsync<int>(
            countActivities,
            parameters,
            transaction: tx());

        var rows = await db.QueryAsync<ActivityEventDto>(
            selectActivities,
            parameters,
            transaction: tx());

        return new PagedResult<ActivityEvent>(
            [.. rows.Select(ActivityEventMappers.ToDomain)],
            totalCount,
            page,
            pageSize);
    }

    public async Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken)
    {
        var createdBefore = DateTimeOffset.FromUnixTimeSeconds(createdBeforeEpochSeconds).UtcDateTime;
        var p = new { CreatedBefore = createdBefore };

        var countstats = "SELECT COUNT(*) from  ActivityEvents WHERE CreatedAt < @CreatedBefore";
        var totalCount = await db.QuerySingleAsync<int>(countstats, p, transaction: tx());

        const string sql = "DELETE FROM ActivityEvents WHERE CreatedAt < @CreatedBefore";
        await db.ExecuteAsync(sql, p, transaction: tx());
        return totalCount;
    }
}
