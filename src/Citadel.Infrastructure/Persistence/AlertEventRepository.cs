using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Hosting.Common.Models;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class AlertEventRepository(IDbConnection db, Func<IDbTransaction> tx) : IAlertEventRepository
{
    public Task<Guid> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertEvents (
            Id, AlertRuleId, Type, Severity, Info, ResourceId, ResourceName, ResourceType, DeduplicationKey, OpenIncidentKey,
            AcknowledgedByActorId, AcknowledgedAt, ResolvedByActorId, ResolvedAt, ResolutionNote, CreatedAt, UpdatedAt
        )
        VALUES (
            @Id, @AlertRuleId, @Type, @Severity, @Info::json, @ResourceId, @ResourceName, @ResourceType, @DeduplicationKey, @OpenIncidentKey,
            @AcknowledgedByActorId, @AcknowledgedAt, @ResolvedByActorId, @ResolvedAt, @ResolutionNote, @CreatedAt, @UpdatedAt
        )
        ON CONFLICT(OpenIncidentKey) DO UPDATE SET
            AlertRuleId = excluded.AlertRuleId,
            Type = excluded.Type,
            Severity = excluded.Severity,
            Info = excluded.Info,
            ResourceId = excluded.ResourceId,
            ResourceName = excluded.ResourceName,
            ResourceType = excluded.ResourceType,
            UpdatedAt = excluded.UpdatedAt
        RETURNING Id";
        return db.ExecuteScalarAsync<Guid>(sql, new
        {
            Id = alertEvent.Id,
            AlertRuleId = alertEvent.AlertRuleId,
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId,
            alertEvent.ResourceName,
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId,
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId,
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            CreatedAt = alertEvent.CreatedAt,
            alertEvent.UpdatedAt
        },
        transaction: tx());
    }

    public async Task<AlertEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceName,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt,
            ac.Id AS Actor_Id,
            COALESCE(au.Name, at.Name, CASE WHEN ac.Type = 'System' THEN 'System' END) AS Actor_Name,
            ac.Type AS Actor_Type
        FROM AlertEvents a
        LEFT JOIN Actors ac ON COALESCE(a.ResolvedByActorId, a.AcknowledgedByActorId) = ac.Id
        LEFT JOIN Users au ON au.ActorId = ac.Id
        LEFT JOIN Teams at ON at.ActorId = ac.Id
        WHERE a.Id = @Id
        LIMIT 1;
        """;

        var row = await db.QuerySingleOrDefaultAsync<AlertEventDto>(sql, new { Id = id }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<AlertEvent?> GetAuthorizedByIdAsync(
        Guid userId,
        ResourceType permissionResourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid id,
        CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.PermissionGlobalAccessCte + @"
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceName,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt,
            ac.Id AS Actor_Id,
            COALESCE(au.Name, at.Name, CASE WHEN ac.Type = 'System' THEN 'System' END) AS Actor_Name,
            ac.Type AS Actor_Type
        FROM AlertEvents a
        LEFT JOIN Actors ac ON COALESCE(a.ResolvedByActorId, a.AcknowledgedByActorId) = ac.Id
        LEFT JOIN Users au ON au.ActorId = ac.Id
        LEFT JOIN Teams at ON at.ActorId = ac.Id
        WHERE a.Id = @Id
            AND " + AuthorizationSql.PermissionResourcePredicatePrefix + "a.Id" + AuthorizationSql.ResourcePredicateSuffix + @"
        LIMIT 1;
        ";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var row = await db.QuerySingleOrDefaultAsync<AlertEventDto>(sql, new
        {
            UserId = userId,
            PermissionResourceType = (int)permissionResourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            Id = id,
            cancellationToken
        }, transaction: tx());

        return row?.ToDomain();
    }

    public async Task<IEnumerable<AlertEvent>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceName,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt,
            ac.Id AS Actor_Id,
            COALESCE(au.Name, at.Name, CASE WHEN ac.Type = 'System' THEN 'System' END) AS Actor_Name,
            ac.Type AS Actor_Type
        FROM AlertEvents a
        LEFT JOIN Actors ac ON COALESCE(a.ResolvedByActorId, a.AcknowledgedByActorId) = ac.Id
        LEFT JOIN Users au ON au.ActorId = ac.Id
        LEFT JOIN Teams at ON at.ActorId = ac.Id
        WHERE a.Id = ANY(@Ids);
        """;

        var rows = await db.QueryAsync<AlertEventDto>(
            sql,
            new { Ids = ids.ToArray() },
            transaction: tx());

        var alertEvents = rows is ICollection<AlertEventDto> rowCollection
            ? new List<AlertEvent>(rowCollection.Count)
            : [];

        foreach (var row in rows)
            alertEvents.Add(row.ToDomain());

        return alertEvents;
    }

    public Task<IEnumerable<Guid>> GetAuthorizedUserIdsAsync(
        Guid alertEventId,
        ResourceType permissionResourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        const string sql = """
        WITH CandidateActorScope AS (
            SELECT u.Id AS UserId, u.ActorId
            FROM Users u
            JOIN Actors userActor ON userActor.Id = u.ActorId
            WHERE userActor.IsEnabled

            UNION

            SELECT u.Id AS UserId, t.ActorId
            FROM Users u
            JOIN Actors userActor ON userActor.Id = u.ActorId
            JOIN UsersTeams ut ON ut.UserId = u.Id
            JOIN Teams t ON t.Id = ut.TeamId
            JOIN Actors teamActor ON teamActor.Id = t.ActorId
            WHERE userActor.IsEnabled
              AND teamActor.IsEnabled
        )
        SELECT DISTINCT u.Id
        FROM Users u
        JOIN Actors userActor ON userActor.Id = u.ActorId
        WHERE userActor.IsEnabled
          AND (
            EXISTS (
                SELECT 1
                FROM CandidateActorScope scope
                JOIN ActorRoles ar ON ar.ActorId = scope.ActorId
                JOIN Roles r ON r.Id = ar.RoleId
                WHERE scope.UserId = u.Id
                  AND LOWER(r.Name) = 'admin'
            )
            OR EXISTS (
                SELECT 1
                FROM CandidateActorScope scope
                JOIN ActorRoles ar ON ar.ActorId = scope.ActorId
                JOIN Permissions p ON p.RoleId = ar.RoleId
                WHERE scope.UserId = u.Id
                  AND p.ResourceType = @PermissionResourceType
                  AND (p.PermissionLevel & @GrantedPermissionMask) <> 0
                  AND (@SpecificPermission = 0 OR (p.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
            )
            OR EXISTS (
                SELECT 1
                FROM CandidateActorScope scope
                JOIN ResourceAccesses ra ON ra.ActorId = scope.ActorId
                WHERE scope.UserId = u.Id
                  AND ra.ResourceType = @PermissionResourceType
                  AND (ra.PermissionLevel & @GrantedPermissionMask) <> 0
                  AND (@SpecificPermission = 0 OR (ra.SpecificPermissions & @SpecificPermission) = @SpecificPermission)
                  AND ra.ResourceId = @AlertEventId
            )
          );
        """;

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        return db.QueryAsync<Guid>(sql, new
        {
            AlertEventId = alertEventId,
            PermissionResourceType = (int)permissionResourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());
    }


    public async Task<PagedResult<AlertEvent>> GetPagedAsync(
     Guid? resourceId,
     AlertType? alertType,
     AlertResourceType? resourceType,
     int page,
     int pageSize,
     CancellationToken cancellationToken,
     bool? unresolvedOnly = null)
    {
        const string SelectAlerts = """
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceName,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt,
            ac.Id AS Actor_Id,
            ac.Type AS Actor_Type
        FROM AlertEvents a
        LEFT JOIN Actors ac ON COALESCE(a.ResolvedByActorId, a.AcknowledgedByActorId) = ac.Id
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR NOT @UnresolvedOnly OR a.ResolvedAt IS NULL)
        ORDER BY a.UpdatedAt DESC, a.CreatedAt DESC
        LIMIT @PageSize OFFSET @Offset;
        """;

        const string CountAlerts = """
        SELECT COUNT(*)
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR NOT @UnresolvedOnly OR a.ResolvedAt IS NULL);
        """;

        var offset = (page - 1) * pageSize;

        var p = new
        {
            ResourceId = resourceId,
            AlertType = alertType is null ? null : EnumFormatter<AlertType>.GetValue(alertType.Value),
            ResourceType = resourceType is null ? null : EnumFormatter<AlertResourceType>.GetValue(resourceType.Value),
            UnresolvedOnly = unresolvedOnly,
            PageSize = pageSize,
            Offset = offset,
        };

        var totalCount = await db.QuerySingleAsync<int>(
            CountAlerts, p, transaction: tx());

        var rows = await db.QueryAsync<AlertEventDto>(
            SelectAlerts, p, transaction: tx());

        return new PagedResult<AlertEvent>(
            [.. rows.Select(x => x.ToDomain())],
            totalCount,
            page,
            pageSize);
    }

    public async Task<PagedResult<AlertEvent>> GetAuthorizedPagedAsync(
        Guid userId,
        ResourceType permissionResourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid? resourceId,
        AlertType? alertType,
        AlertResourceType? resourceType,
        int page,
        int pageSize,
        CancellationToken cancellationToken,
        bool? unresolvedOnly = null)
    {
        const string selectAlerts = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.PermissionGlobalAccessCte + @"
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceName,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt,
            ac.Id AS Actor_Id,
            ac.Type AS Actor_Type
        FROM AlertEvents a
        LEFT JOIN Actors ac ON COALESCE(a.ResolvedByActorId, a.AcknowledgedByActorId) = ac.Id
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR NOT @UnresolvedOnly OR a.ResolvedAt IS NULL)
            AND " + AuthorizationSql.PermissionResourcePredicatePrefix + "a.Id" + AuthorizationSql.ResourcePredicateSuffix + @"
        ORDER BY a.UpdatedAt DESC, a.CreatedAt DESC
        LIMIT @PageSize OFFSET @Offset;
        ";

        const string countAlerts = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.PermissionGlobalAccessCte + @"
        SELECT COUNT(*)
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR NOT @UnresolvedOnly OR a.ResolvedAt IS NULL)
            AND " + AuthorizationSql.PermissionResourcePredicatePrefix + "a.Id" + AuthorizationSql.ResourcePredicateSuffix + ";";

        var offset = (page - 1) * pageSize;
        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);

        var p = new
        {
            UserId = userId,
            PermissionResourceType = (int)permissionResourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            ResourceId = resourceId,
            AlertType = alertType is null ? null : EnumFormatter<AlertType>.GetValue(alertType.Value),
            ResourceType = resourceType is null ? null : EnumFormatter<AlertResourceType>.GetValue(resourceType.Value),
            UnresolvedOnly = unresolvedOnly,
            PageSize = pageSize,
            Offset = offset,
        };

        var totalCount = await db.QuerySingleAsync<int>(countAlerts, p, transaction: tx());
        var rows = await db.QueryAsync<AlertEventDto>(selectAlerts, p, transaction: tx());

        return new PagedResult<AlertEvent>(
            [.. rows.Select(x => x.ToDomain())],
            totalCount,
            page,
            pageSize);
    }

    public Task<int> UpdateAsync(AlertEvent alertEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertEvents
        SET
            AlertRuleId = @AlertRuleId,
            Type = @Type,
            Severity = @Severity,
            Info = @Info::json,
            ResourceId = @ResourceId,
            ResourceName = @ResourceName,
            ResourceType = @ResourceType,
            DeduplicationKey = @DeduplicationKey,
            OpenIncidentKey = @OpenIncidentKey,
            AcknowledgedByActorId = @AcknowledgedByActorId,
            AcknowledgedAt = @AcknowledgedAt,
            ResolvedByActorId = @ResolvedByActorId,
            ResolvedAt = @ResolvedAt,
            ResolutionNote = @ResolutionNote,
            UpdatedAt = @UpdatedAt
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, new
        {
            Id = alertEvent.Id,
            AlertRuleId = alertEvent.AlertRuleId,
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId,
            alertEvent.ResourceName,
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId,
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId,
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            alertEvent.UpdatedAt
        }, transaction: tx());
    }

    public Task<int> BulkUpdateAsync(IEnumerable<AlertEvent> alertEvents, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertEvents
        SET
            AlertRuleId = @AlertRuleId,
            Type = @Type,
            Severity = @Severity,
            Info = @Info::json,
            ResourceId = @ResourceId,
            ResourceName = @ResourceName,
            ResourceType = @ResourceType,
            DeduplicationKey = @DeduplicationKey,
            OpenIncidentKey = @OpenIncidentKey,
            AcknowledgedByActorId = @AcknowledgedByActorId,
            AcknowledgedAt = @AcknowledgedAt,
            ResolvedByActorId = @ResolvedByActorId,
            ResolvedAt = @ResolvedAt,
            ResolutionNote = @ResolutionNote,
            UpdatedAt = @UpdatedAt
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, alertEvents.Select(alertEvent => new
        {
            Id = alertEvent.Id,
            AlertRuleId = alertEvent.AlertRuleId,
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId,
            alertEvent.ResourceName,
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId,
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId,
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            alertEvent.UpdatedAt
        }), transaction: tx());
    }

    public Task<int> CountUnresolvedAsync(CancellationToken cancellationToken)
        => db.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM AlertEvents WHERE ResolvedAt IS NULL",
            transaction: tx());

    public Task<int> CountAuthorizedUnresolvedAsync(
        Guid userId,
        ResourceType permissionResourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.PermissionGlobalAccessCte + @",
        AdminAccess AS (
            SELECT 1 AS HasAccess
            FROM ActorRoles ar
            JOIN Roles r ON r.Id = ar.RoleId
            JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
            WHERE LOWER(r.Name) = 'admin'
            LIMIT 1
        )
        SELECT COUNT(*)
        FROM AlertEvents a
        WHERE a.ResolvedAt IS NULL
            AND (
                EXISTS (SELECT 1 FROM AdminAccess)
                OR " + AuthorizationSql.PermissionResourcePredicatePrefix + "a.Id" + AuthorizationSql.ResourcePredicateSuffix + @"
            );";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        return db.QuerySingleAsync<int>(sql, new
        {
            UserId = userId,
            PermissionResourceType = (int)permissionResourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());
    }
}
