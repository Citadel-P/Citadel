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

internal sealed class AlertRuleRepository(IDbConnection db, Func<IDbTransaction> tx) : IAlertRuleRepository
{
    public async Task<AlertRule?> GetByIdAsync(Guid alertRuleId, CancellationToken cancellationToken)
    {
        const string query = """
        SELECT 
            r.Id, 
            r.Url,
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

        var result = await db.QuerySingleOrDefaultAsync<AlertRuleDto>(query, new { Id = alertRuleId.Format() }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertRules (
            Id, Url, Type, CooldownSeconds, IsEnabled, Scope, Severity, LimitedTo, QuietHours, RequiredMatches, Threshold, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @Url, @Type, @CooldownSeconds, @IsEnabled, @Scope, @Severity, @LimitedTo, @QuietHours, @RequiredMatches, @Threshold, @CreatedByActorId, @CreatedAt
        )";
        return db.ExecuteAsync(sql, new
        {
            Id = alertRule.Id.Format(),
            Url = alertRule.Url,
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
    }

    public Task<int> AddAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertRuleStates (
            AlertRuleId, ResourceId, ConsecutiveMatches, LastTriggeredAt, CreatedByActorId, CreatedAt
        )
        VALUES (
            @AlertRuleId, @ResourceId, @ConsecutiveMatches, @LastTriggeredAt, @CreatedByActorId, @CreatedAt
        )";
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
        const string query= """
        SELECT 
            r.Id, 
            r.Url,
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

        var rows = await db.QueryAsync<AlertRuleDto>(query, transaction: tx());
        return rows?.ToDomain() ?? [];
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

    public Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertRules
        SET
            Url = @Url,
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
        return db.ExecuteAsync(sql, new
        {
            Id = alertRule.Id.Format(),
            Url = alertRule.Url,
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
    }

    public Task<int> UpdateStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertRuleStates
        SET
            ConsecutiveMatches = @ConsecutiveMatches,
            LastTriggeredAt = @LastTriggeredAt
        WHERE AlertRuleId = @AlertRuleId
          AND ResourceId = @ResourceId";

        return db.ExecuteAsync(sql, new
        {
            AlertRuleId = alertRuleState.AlertRuleId.Format(),
            ResourceId = alertRuleState.ResourceId.Format(),
            ConsecutiveMatches = alertRuleState.ConsecutiveMatches,
            LastTriggeredAt = alertRuleState.LastTriggeredAt
        },
        transaction: tx());
    }
}
