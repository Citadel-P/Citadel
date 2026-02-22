using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
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
        const string ruleQuery = """
        SELECT 
            r.Id, 
            r.Type, 
            r.CooldownSeconds, 
            r.IsEnabled, 
            r.Scope, 
            r.Severity, 
            r.LimitedTo, 
            r.QuietHours, 
            r.RequiredMatches,
            r.Threshold, 
            r.CreatedByActorId, 
            r.CreatedAt,
            s.ResourceId AS State_ResourceId,
            s.ConsecutiveMatches AS State_ConsecutiveMatches,
            s.LastTriggeredAt AS State_LastTriggeredAt,
            s.CreatedByActorId AS State_CreatedByActorId,
            s.CreatedAt AS State_CreatedAt
        FROM AlertRules r
        LEFT JOIN AlertRuleStates s ON r.Id = s.AlertRuleId
        WHERE r.Id = @Id
        LIMIT 1
        """;

        const string channelQuery = """
        SELECT c.Id, arc.AlertRuleId, c.AlertDestination, c.Url, c.IsActive, c.CreatedByActorId, c.CreatedAt
        FROM AlertChannels c
        INNER JOIN AlertRuleChannels arc ON c.Id = arc.AlertChannelId
        WHERE arc.AlertRuleId = @Id
        """;

        var id = alertRuleId.Format();
        var result = await db.QuerySingleOrDefaultAsync<AlertRuleDto>(ruleQuery, new { Id = id }, transaction: tx());
        if (result is null)
            return null;

        var channels = await db.QueryAsync<AlertChannelDto>(channelQuery, new { Id = id }, transaction: tx());
        result = result with { Channels = channels.ToList() };

        return result.ToDomain();
    }

    public async Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string ruleSql = @"
        INSERT INTO AlertRules (
            Id, Type, CooldownSeconds, IsEnabled, Scope, Severity, LimitedTo, QuietHours, RequiredMatches, Threshold, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @Type, @CooldownSeconds, @IsEnabled, @Scope, @Severity, @LimitedTo, @QuietHours, @RequiredMatches, @Threshold, @CreatedByActorId, @CreatedAt
        )";

        var ruleId = alertRule.Id.Format();
        var rows = await db.ExecuteAsync(ruleSql, new
        {
            Id = ruleId,
            Type = EnumFormatter<AlertType>.GetValue(alertRule.Type),
            CooldownSeconds = alertRule.CooldownSeconds,
            IsEnabled = alertRule.IsEnabled,
            RequiredMatches = alertRule.RequiredMatches,
            Threshold = alertRule.Threshold,
            Scope = EnumFormatter<AlertScope>.GetValue(alertRule.Scope),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertRule.Severity),
            QuietHours = JsonSerializer.Serialize(alertRule.QuietHours, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour),
            LimitedTo = JsonSerializer.Serialize(alertRule.LimitedTo, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo),
            CreatedByActorId = alertRule.CreatedByActorId.Format(),
            CreatedAt = alertRule.CreatedAt
        },
        transaction: tx());

        if (alertRule.Channels.Count > 0)
            await InsertChannelsAsync(ruleId, alertRule.Channels);

        return rows;
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
            AlertRuleId = alertRuleState.AlertRuleId.Format(),
            ResourceId = alertRuleState.ResourceId.Format(),
            ConsecutiveMatches = alertRuleState.ConsecutiveMatches,
            LastTriggeredAt = alertRuleState.LastTriggeredAt,
            CreatedByActorId = alertRuleState.CreatedByActorId.Format(),
            CreatedAt = alertRuleState.CreatedAt
        },
        transaction: tx());
    }

    public async Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        const string ruleQuery = """
        SELECT 
            r.Id, 
            r.Type, 
            r.CooldownSeconds, 
            r.IsEnabled, 
            r.Scope, 
            r.Severity, 
            r.LimitedTo, 
            r.QuietHours, 
            r.RequiredMatches,
            r.Threshold, 
            r.CreatedByActorId, 
            r.CreatedAt,
            s.ResourceId AS State_ResourceId,
            s.ConsecutiveMatches AS State_ConsecutiveMatches,
            s.LastTriggeredAt AS State_LastTriggeredAt,
            s.CreatedByActorId AS State_CreatedByActorId,
            s.CreatedAt AS State_CreatedAt
        FROM AlertRules r
        LEFT JOIN AlertRuleStates s ON r.Id = s.AlertRuleId
        """;

        const string channelQuery = """
        SELECT c.Id, arc.AlertRuleId, c.AlertDestination, c.Url, c.IsActive, c.CreatedByActorId, c.CreatedAt
        FROM AlertChannels c
        INNER JOIN AlertRuleChannels arc ON c.Id = arc.AlertChannelId
        """;

        var rows = (await db.QueryAsync<AlertRuleDto>(ruleQuery, transaction: tx()))?.ToList();
        if (rows is null || rows.Count == 0)
            return [];

        var channels = (await db.QueryAsync<AlertChannelDto>(channelQuery, transaction: tx()))
            .GroupBy(c => c.AlertRuleId)
            .ToDictionary(g => g.Key, g => g.ToList());

        for (var i = 0; i < rows.Count; i++)
        {
            if (channels.TryGetValue(rows[i].Id, out var ruleChannels))
                rows[i] = rows[i] with { Channels = ruleChannels };
        }

        return rows.ToDomain();
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
                AlertRuleId = alertRuleId.Format(),
                ResourceId = resourceId.Format()
            },
            transaction: tx());

        return dto?.ToDomain();
    }

    public async Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string ruleSql = @"
        UPDATE AlertRules
        SET
            Type = @Type,
            CooldownSeconds = @CooldownSeconds,
            IsEnabled = @IsEnabled,
            Scope = @Scope,
            Severity = @Severity,
            LimitedTo = @LimitedTo,
            QuietHours = @QuietHours,
            RequiredMatches = @RequiredMatches,
            Threshold = @Threshold
        WHERE Id = @Id";

        const string unlinkChannelsSql = @"DELETE FROM AlertRuleChannels WHERE AlertRuleId = @AlertRuleId";

        var ruleId = alertRule.Id.Format();
        var rows = await db.ExecuteAsync(ruleSql, new
        {
            Id = ruleId,
            Type = EnumFormatter<AlertType>.GetValue(alertRule.Type),
            CooldownSeconds = alertRule.CooldownSeconds,
            IsEnabled = alertRule.IsEnabled,
            Scope = EnumFormatter<AlertScope>.GetValue(alertRule.Scope),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertRule.Severity),
            QuietHours = JsonSerializer.Serialize(alertRule.QuietHours, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleQuietHour),
            LimitedTo = JsonSerializer.Serialize(alertRule.LimitedTo, AlertRuleJsonContext.Default.IReadOnlyCollectionAlertRuleLimitedTo),
            RequiredMatches = alertRule.RequiredMatches,
            Threshold = alertRule.Threshold
        },
        transaction: tx());

        await db.ExecuteAsync(unlinkChannelsSql, new { AlertRuleId = ruleId }, transaction: tx());

        if (alertRule.Channels.Count > 0)
            await InsertChannelsAsync(ruleId, alertRule.Channels);

        return rows;
    }

    public Task<int> UpdateChannelAsync(AlertChannel channel, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertChannels
        SET
            AlertDestination = @AlertDestination,
            Url = @Url,
            IsActive = @IsActive
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, new
        {
            Id = channel.Id.Format(),
            AlertDestination = EnumFormatter<AlertDestination>.GetValue(channel.AlertDestination),
            Url = channel.Url,
            IsActive = channel.IsActive
        }, transaction: tx());
    }

    private async Task InsertChannelsAsync(string alertRuleId, IEnumerable<AlertChannel> channels)
    {
        const string upsertChannelSql = @"
        INSERT INTO AlertChannels (Id, AlertDestination, Url, IsActive, CreatedByActorId, CreatedAt)
        VALUES (@Id, @AlertDestination, @Url, @IsActive, @CreatedByActorId, @CreatedAt)
        ON CONFLICT(Id) DO UPDATE SET
            AlertDestination = excluded.AlertDestination,
            Url = excluded.Url,
            IsActive = excluded.IsActive";

        const string linkSql = @"
        INSERT INTO AlertRuleChannels (AlertRuleId, AlertChannelId)
        VALUES (@AlertRuleId, @AlertChannelId)
        ON CONFLICT(AlertRuleId, AlertChannelId) DO NOTHING";

        var channelList = channels.ToList();

        await db.ExecuteAsync(upsertChannelSql, channelList.Select(c => new
        {
            Id = c.Id.Format(),
            AlertDestination = EnumFormatter<AlertDestination>.GetValue(c.AlertDestination),
            Url = c.Url,
            IsActive = c.IsActive,
            CreatedByActorId = c.CreatedByActorId.Format(),
            CreatedAt = c.CreatedAt
        }), transaction: tx());

        await db.ExecuteAsync(linkSql, channelList.Select(c => new
        {
            AlertRuleId = alertRuleId,
            AlertChannelId = c.Id.Format()
        }), transaction: tx());
    }
}
