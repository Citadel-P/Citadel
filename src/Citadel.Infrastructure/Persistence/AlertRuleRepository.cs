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

internal sealed class AlertRuleRepository(IDbConnection db, Func<IDbTransaction> tx) : IAlertRuleRepository
{
    public async Task<AlertRule?> GetByIdAsync(Guid alertRuleId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                r.Id, 
                r.Name,
                r.Description,
                r.Type, 
                r.CooldownSeconds, 
                r.Status, 
                r.Severity, 
                r.LimitedTo, 
                r.QuietHours, 
                r.RequiredMatches,
                r.Threshold, 
                r.CreatedByActorId, 
                r.CreatedAt,
                COALESCE(json_agg(arc.AlertChannelId) FILTER (WHERE arc.AlertChannelId IS NOT NULL), '[]'::json)::text AS ChannelIds
            FROM AlertRules r
            LEFT JOIN AlertRuleChannels arc ON arc.AlertRuleId = r.Id
            WHERE r.Id = @Id
            GROUP BY r.Id
            LIMIT 1
            """;

        var id = alertRuleId;

        var dto = await db.QuerySingleOrDefaultAsync<AlertRuleDto>(
            sql,
            new { Id = id },
            transaction: tx());

        if (dto is null)
            return null;

        return dto.ToDomain();
    }

    public async Task<IEnumerable<AlertRule>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT
            r.Id, 
            r.Name,
            r.Description,
            r.Type, 
            r.CooldownSeconds, 
            r.Status, 
            r.Severity, 
            r.LimitedTo, 
            r.QuietHours, 
            r.RequiredMatches,
            r.Threshold, 
            r.CreatedByActorId, 
            r.CreatedAt,
            COALESCE(json_agg(arc.AlertChannelId) FILTER (WHERE arc.AlertChannelId IS NOT NULL), '[]'::json)::text AS ChannelIds
        FROM AlertRules r
        LEFT JOIN AlertRuleChannels arc ON arc.AlertRuleId = r.Id
            WHERE r.Id = ANY(@Ids)
            GROUP BY r.Id
           
    """;
        var result = await db.QueryAsync<AlertRuleDto>(sql, new { Ids = ids.ToArray(), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string ruleSql = @"
        INSERT INTO AlertRules (
            Id, Name, Description, Type, CooldownSeconds, Status, Severity, LimitedTo, QuietHours, RequiredMatches, Threshold, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @Name, @Description, @Type, @CooldownSeconds, @Status, @Severity, @LimitedTo::json, @QuietHours::json, @RequiredMatches, @Threshold, @CreatedByActorId, @CreatedAt
        )";

        var ruleId = alertRule.Id;
        var rows = await db.ExecuteAsync(ruleSql, new
        {
            Id = ruleId,
            Name = alertRule.Name,
            Description = alertRule.Description,
            Type = EnumFormatter<AlertType>.GetValue(alertRule.Type),
            CooldownSeconds = alertRule.CooldownSeconds,
            Status = EnumFormatter<AlertRuleStatus>.GetValue(alertRule.Status),
            RequiredMatches = alertRule.RequiredMatches,
            Threshold = alertRule.Threshold,
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertRule.Severity),
            QuietHours = JsonSerializer.Serialize(alertRule.QuietHours, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour),
            LimitedTo = JsonSerializer.Serialize(alertRule.LimitedTo, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo),
            CreatedByActorId = alertRule.CreatedByActorId,
            CreatedAt = alertRule.CreatedAt
        },
        transaction: tx());

        if (alertRule.ChannelIds.Count > 0)
            await LinkChannelsAsync(ruleId, alertRule.ChannelIds);

        return rows;
    }

    public Task<int> AddChannelAsync(AlertChannel alertChannel, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertChannels (
            Id, Name, AlertDestination, Url, IsActive, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @Name, @AlertDestination, @Url, @IsActive, @CreatedByActorId, @CreatedAt
        )";

        return db.ExecuteAsync(sql, new
        {
            Id = alertChannel.Id,
            Name = alertChannel.Name,
            AlertDestination = EnumFormatter<AlertDestination>.GetValue(alertChannel.AlertDestination),
            Url = alertChannel.Url,
            IsActive = alertChannel.IsActive,
            CreatedByActorId = alertChannel.CreatedByActorId,
            CreatedAt = alertChannel.CreatedAt,
        }, transaction: tx());
    }

    public Task<int> UpsertAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertRuleStates (
            AlertRuleId, ResourceId, ConsecutiveMatches, LastTriggeredAt, CreatedByActorId, CreatedAt
        )
        VALUES (
            @AlertRuleId, @ResourceId, @ConsecutiveMatches, @LastTriggeredAt, @CreatedByActorId, @CreatedAt
        )
        ON CONFLICT(AlertRuleId, ResourceId) DO UPDATE SET
            ConsecutiveMatches = excluded.ConsecutiveMatches,
            LastTriggeredAt = excluded.LastTriggeredAt
        ";
        return db.ExecuteAsync(sql, new
        {
            AlertRuleId = alertRuleState.AlertRuleId,
            ResourceId = alertRuleState.ResourceId,
            ConsecutiveMatches = alertRuleState.ConsecutiveMatches,
            LastTriggeredAt = alertRuleState.LastTriggeredAt,
            CreatedByActorId = alertRuleState.CreatedByActorId,
            CreatedAt = alertRuleState.CreatedAt
        },
        transaction: tx());
    }

    public async Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        const string sql = """
            SELECT 
                r.Id, 
                r.Name,
                r.Description,
                r.Type, 
                r.CooldownSeconds, 
                r.Status, 
                r.Severity, 
                r.LimitedTo, 
                r.QuietHours, 
                r.RequiredMatches,
                r.Threshold, 
                r.CreatedByActorId, 
                r.CreatedAt,
                COALESCE(json_agg(arc.AlertChannelId) FILTER (WHERE arc.AlertChannelId IS NOT NULL), '[]'::json)::text AS ChannelIds
            FROM AlertRules r
            LEFT JOIN AlertRuleChannels arc ON arc.AlertRuleId = r.Id
            GROUP BY r.Id
        """;

        var rows = await db.QueryAsync<AlertRuleDto>(
            sql,
            transaction: tx());

        if (rows is null)
            return [];

        return rows.ToDomain();
    }

    public async Task<IEnumerable<AlertRule>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + @"
            SELECT 
                r.Id, 
                r.Name,
                r.Description,
                r.Type, 
                r.CooldownSeconds, 
                r.Status, 
                r.Severity, 
                r.LimitedTo, 
                r.QuietHours, 
                r.RequiredMatches,
                r.Threshold, 
                r.CreatedByActorId, 
                r.CreatedAt,
                COALESCE(json_agg(arc.AlertChannelId) FILTER (WHERE arc.AlertChannelId IS NOT NULL), '[]'::json)::text AS ChannelIds
            FROM AlertRules r
            LEFT JOIN AlertRuleChannels arc ON arc.AlertRuleId = r.Id
            WHERE " + AuthorizationSql.ResourcePredicatePrefix + "r.Id" + AuthorizationSql.ResourcePredicateSuffix + @"
            GROUP BY r.Id
            ";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var rows = await db.QueryAsync<AlertRuleDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());

        return rows?.ToDomain() ?? [];
    }

    public Task<bool> CanAccessAsync(Guid userId, Guid alertRuleId, CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}} SELECT EXISTS (SELECT 1 FROM AlertRules r WHERE r.Id = @AlertRuleId AND 
            {{AuthorizationSql.ResourcePredicatePrefix}}r.Id{{AuthorizationSql.ResourcePredicateSuffix}})
        """;

        return db.ExecuteScalarAsync<bool>(sql, new
        {
            UserId = userId,
            AlertRuleId = alertRuleId,
            ResourceType = (int)ResourceType.Alert,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
            SpecificPermission = (int)SpecificPermission.None,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<PagedResult<AlertRule>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken)
    {
        const string countQuery = "SELECT COUNT(*) FROM AlertRules";

        const string sql = """
            SELECT 
                r.Id,
                r.Name,
                r.Description,
                r.Type,
                r.CooldownSeconds,
                r.Status,
                r.Severity,
                r.LimitedTo,
                r.QuietHours,
                r.RequiredMatches,
                r.Threshold,
                r.CreatedByActorId,
                r.CreatedAt,
                COALESCE(json_agg(arc.AlertChannelId) FILTER (WHERE arc.AlertChannelId IS NOT NULL), '[]'::json)::text AS ChannelIds
            FROM (
                SELECT *
                FROM AlertRules
                ORDER BY CreatedAt DESC, Type DESC
                LIMIT @PageSize OFFSET @Offset
            ) r
            LEFT JOIN AlertRuleChannels arc ON arc.AlertRuleId = r.Id
            GROUP BY r.Id
            ORDER BY r.CreatedAt DESC, r.Type DESC
         """;

        var totalCount = await db.ExecuteScalarAsync<int>(
            countQuery,
            transaction: tx());

        var offset = (page - 1) * pageSize;

        var rows = await db.QueryAsync<AlertRuleDto>(
            sql,
            new { PageSize = pageSize, Offset = offset },
            transaction: tx());

        if (rows is null)
            return new PagedResult<AlertRule>([], totalCount, page, pageSize);

        return new PagedResult<AlertRule>(
            rows.ToDomain(),
            totalCount,
            page,
            pageSize);
    }

    public async Task<AlertChannel?> GetChannelByIdAsync(Guid channelId, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT Id, Name, AlertDestination, Url, IsActive, CreatedByActorId, CreatedAt
        FROM AlertChannels
        WHERE Id = @Id
        LIMIT 1
        """;

        var dto = await db.QuerySingleOrDefaultAsync<AlertChannelDto>(sql, new { Id = channelId }, transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<IEnumerable<AlertChannel>> GetAllChannelsAsync(CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT Id, Name, AlertDestination, Url, IsActive, CreatedByActorId, CreatedAt
        FROM AlertChannels
        """;

        var rows = await db.QueryAsync<AlertChannelDto>(sql, transaction: tx());
        return rows?.Select(r => r.ToDomain()) ?? [];
    }

    public async Task<IEnumerable<AlertChannel>> GetAuthorizedChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " " + """
            SELECT Id, Name, AlertDestination, Url, IsActive, CreatedByActorId, CreatedAt
            FROM AlertChannels channel
            WHERE
        """ + AuthorizationSql.ResourcePredicatePrefix + "channel.Id" + AuthorizationSql.ResourcePredicateSuffix + ";";

        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var rows = await db.QueryAsync<AlertChannelDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());

        return rows?.Select(r => r.ToDomain()) ?? [];
    }

    public async Task<AlertRuleState?> GetStateAsync(Guid alertRuleId, Guid resourceId, CancellationToken cancellationToken)
    {
        const string sql = @"
        SELECT
            AlertRuleId,
            ResourceId,
            ConsecutiveMatches,
            LastTriggeredAt,
            CreatedByActorId,
            CreatedAt
        FROM AlertRuleStates
        WHERE AlertRuleId = @AlertRuleId
          AND ResourceId = @ResourceId
        LIMIT 1";

        var dto = await db.QuerySingleOrDefaultAsync<AlertRuleStateDto>(
            sql,
            new
            {
                AlertRuleId = alertRuleId,
                ResourceId = resourceId
            },
            transaction: tx());

        return dto?.ToDomain();
    }

    public async Task<IEnumerable<AlertChannel>> GetAuthorizedAlertChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + @"
            SELECT ac.Id, ac.Name, ac.AlertDestination, ac.Url, ac.IsActive, ac.CreatedByActorId, ac.CreatedAt
            FROM AlertChannels ac
            WHERE " + AuthorizationSql.ResourcePredicatePrefix + "ac.Id" + AuthorizationSql.ResourcePredicateSuffix;
        var grantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel);
        var rows = await db.QueryAsync<AlertChannelDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = grantedPermissionMask,
            SpecificPermission = (int)specificPermission,
            cancellationToken
        }, transaction: tx());
        return rows?.Select(r => r.ToDomain()) ?? [];
    }

    public async Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string ruleSql = @"
        UPDATE AlertRules
        SET
            Name = @Name,
            Description = @Description,
            Type = @Type,
            CooldownSeconds = @CooldownSeconds,
            Status = @Status,
            Severity = @Severity,
            LimitedTo = @LimitedTo::json,
            QuietHours = @QuietHours::json,
            RequiredMatches = @RequiredMatches,
            Threshold = @Threshold
        WHERE Id = @Id";

        const string unlinkChannelsSql = @"DELETE FROM AlertRuleChannels WHERE AlertRuleId = @AlertRuleId";

        var ruleId = alertRule.Id;
        var rows = await db.ExecuteAsync(ruleSql, new
        {
            Id = ruleId,
            Name = alertRule.Name,
            Description = alertRule.Description,
            Type = EnumFormatter<AlertType>.GetValue(alertRule.Type),
            CooldownSeconds = alertRule.CooldownSeconds,
            Status = EnumFormatter<AlertRuleStatus>.GetValue(alertRule.Status),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertRule.Severity),
            QuietHours = JsonSerializer.Serialize(alertRule.QuietHours, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour),
            LimitedTo = JsonSerializer.Serialize(alertRule.LimitedTo, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo),
            RequiredMatches = alertRule.RequiredMatches,
            Threshold = alertRule.Threshold
        },
        transaction: tx());

        await db.ExecuteAsync(unlinkChannelsSql, new { AlertRuleId = ruleId }, transaction: tx());

        if (alertRule.ChannelIds.Count > 0)
            await LinkChannelsAsync(ruleId, alertRule.ChannelIds);

        return rows;
    }

    public Task<int> UpdateChannelAsync(AlertChannel channel, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertChannels
        SET
            Name = @Name,
            AlertDestination = @AlertDestination,
            Url = @Url,
            IsActive = @IsActive
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, new
        {
            Id = channel.Id,
            Name = channel.Name,
            AlertDestination = EnumFormatter<AlertDestination>.GetValue(channel.AlertDestination),
            Url = channel.Url,
            IsActive = channel.IsActive
        }, transaction: tx());
    }

    public async Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string unlinkSql = @"DELETE FROM AlertRuleChannels WHERE AlertRuleId = ANY(@Ids)";
        const string deleteStatesSql = @"DELETE FROM AlertRuleStates WHERE AlertRuleId = ANY(@Ids)";
        const string deleteSql = @"DELETE FROM AlertRules WHERE Id = ANY(@Ids)";

        await db.ExecuteAsync(unlinkSql, new { Ids = ids.ToArray() }, transaction: tx());
        await db.ExecuteAsync(deleteStatesSql, new { Ids = ids.ToArray() }, transaction: tx());
        return await db.ExecuteAsync(deleteSql, new { Ids = ids.ToArray() }, transaction: tx());
    }

    public Task<int> RemoveChannelsRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = @"DELETE FROM AlertChannels WHERE Id = ANY(@Ids)";
        var idsArray = ids.ToArray();
        return db.ExecuteAsync(sql, new { Ids = idsArray }, transaction: tx());
    }

    private Task<int> LinkChannelsAsync(Guid alertRuleId, IEnumerable<Guid> channelIds)
    {
        const string linkSql = @"
        INSERT INTO AlertRuleChannels (AlertRuleId, AlertChannelId)
        VALUES (@AlertRuleId, @AlertChannelId)
        ON CONFLICT(AlertRuleId, AlertChannelId) DO NOTHING";

        return db.ExecuteAsync(linkSql, channelIds.Select(channelId => new
        {
            AlertRuleId = alertRuleId,
            AlertChannelId = channelId
        }), transaction: tx());
    }
}
