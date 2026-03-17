using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Models;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class AlertEventRepository(IDbConnection db, Func<IDbTransaction> tx) : IAlertEventRepository
{
    public Task<int> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO AlertEvents (
            Id, AlertRuleId, CreatedByActorId, Type, Severity, Info, ResourceId, ResourceType, DeduplicationKey, OpenIncidentKey,
            AcknowledgedByActorId, AcknowledgedAt, ResolvedByActorId, ResolvedAt, ResolutionNote, CreatedAt, UpdatedAt
        )
        VALUES (
            @Id, @AlertRuleId, @CreatedByActorId, @Type, @Severity, @Info, @ResourceId, @ResourceType, @DeduplicationKey, @OpenIncidentKey,
            @AcknowledgedByActorId, @AcknowledgedAt, @ResolvedByActorId, @ResolvedAt, @ResolutionNote, @CreatedAt, @UpdatedAt
        )
        ON CONFLICT(OpenIncidentKey) DO UPDATE SET
            AlertRuleId = excluded.AlertRuleId,
            CreatedByActorId = excluded.CreatedByActorId,
            Type = excluded.Type,
            Severity = excluded.Severity,
            Info = excluded.Info,
            ResourceId = excluded.ResourceId,
            ResourceType = excluded.ResourceType,
            UpdatedAt = excluded.UpdatedAt";
        return db.ExecuteAsync(sql, new
        {
            Id = alertEvent.Id.Format(),
            AlertRuleId = alertEvent.AlertRuleId.Format(),
            CreatedByActorId = alertEvent.CreatedByActorId.Format(),
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId?.Format(),
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId?.Format(),
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId?.Format(),
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
            a.CreatedByActorId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt
        FROM AlertEvents a
        WHERE a.Id = @Id
        LIMIT 1;
        """;

        var row = await db.QuerySingleOrDefaultAsync<AlertEventDto>(sql, new { Id = id.Format() }, transaction: tx());
        return row?.ToDomain();
    }

    public async Task<IEnumerable<AlertEvent>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
        SELECT
            a.Id,
            a.AlertRuleId,
            a.CreatedByActorId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt
        FROM AlertEvents a
        WHERE a.Id IN (SELECT value FROM json_each(@Ids));
        """;

        var rows = await db.QueryAsync<AlertEventDto>(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx());

        return rows.Select(x => x.ToDomain());
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
            a.CreatedByActorId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceType,
            a.DeduplicationKey,
            a.OpenIncidentKey,
            a.AcknowledgedByActorId,
            a.AcknowledgedAt,
            a.ResolvedByActorId,
            a.ResolvedAt,
            a.ResolutionNote,
            a.CreatedAt,
            a.UpdatedAt
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR @UnresolvedOnly = 0 OR a.ResolvedAt IS NULL)
        ORDER BY a.UpdatedAt DESC, a.CreatedAt DESC
        LIMIT @PageSize OFFSET @Offset;
        """;

        const string CountAlerts = """
        SELECT COUNT(*)
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
            AND (@UnresolvedOnly IS NULL OR @UnresolvedOnly = 0 OR a.ResolvedAt IS NULL);
        """;

        var offset = (page - 1) * pageSize;

        var p = new
        {
            ResourceId = resourceId?.Format(),
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

    public Task<int> UpdateAsync(AlertEvent alertEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertEvents
        SET
            AlertRuleId = @AlertRuleId,
            CreatedByActorId = @CreatedByActorId,
            Type = @Type,
            Severity = @Severity,
            Info = @Info,
            ResourceId = @ResourceId,
            ResourceType = @ResourceType,
            DeduplicationKey = @DeduplicationKey,
            OpenIncidentKey = @OpenIncidentKey,
            AcknowledgedByActorId = @AcknowledgedByActorId,
            AcknowledgedAt = @AcknowledgedAt,
            ResolvedByActorId = @ResolvedByActorId,
            ResolvedAt = @ResolvedAt,
            ResolutionNote = @ResolutionNote,
            CreatedAt = @CreatedAt,
            UpdatedAt = @UpdatedAt
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, new
        {
            Id = alertEvent.Id.Format(),
            AlertRuleId = alertEvent.AlertRuleId.Format(),
            CreatedByActorId = alertEvent.CreatedByActorId.Format(),
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId?.Format(),
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId?.Format(),
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId?.Format(),
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            alertEvent.CreatedAt,
            alertEvent.UpdatedAt
        }, transaction: tx());
    }

    public Task<int> BulkUpdateAsync(IEnumerable<AlertEvent> alertEvents, CancellationToken cancellationToken)
    {
        const string sql = @"
        UPDATE AlertEvents
        SET
            AlertRuleId = @AlertRuleId,
            CreatedByActorId = @CreatedByActorId,
            Type = @Type,
            Severity = @Severity,
            Info = @Info,
            ResourceId = @ResourceId,
            ResourceType = @ResourceType,
            DeduplicationKey = @DeduplicationKey,
            OpenIncidentKey = @OpenIncidentKey,
            AcknowledgedByActorId = @AcknowledgedByActorId,
            AcknowledgedAt = @AcknowledgedAt,
            ResolvedByActorId = @ResolvedByActorId,
            ResolvedAt = @ResolvedAt,
            ResolutionNote = @ResolutionNote,
            CreatedAt = @CreatedAt,
            UpdatedAt = @UpdatedAt
        WHERE Id = @Id";

        return db.ExecuteAsync(sql, alertEvents.Select(alertEvent => new
        {
            Id = alertEvent.Id.Format(),
            AlertRuleId = alertEvent.AlertRuleId.Format(),
            CreatedByActorId = alertEvent.CreatedByActorId.Format(),
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertEventInfo),
            ResourceId = alertEvent.ResourceId?.Format(),
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            alertEvent.DeduplicationKey,
            alertEvent.OpenIncidentKey,
            AcknowledgedByActorId = alertEvent.AcknowledgedByActorId?.Format(),
            alertEvent.AcknowledgedAt,
            ResolvedByActorId = alertEvent.ResolvedByActorId?.Format(),
            alertEvent.ResolvedAt,
            alertEvent.ResolutionNote,
            alertEvent.CreatedAt,
            alertEvent.UpdatedAt
        }), transaction: tx());
    }

    public Task<int> CountUnresolvedAsync(CancellationToken cancellationToken)
        => db.QuerySingleAsync<int>(
            "SELECT COUNT(*) FROM AlertEvents WHERE ResolvedAt IS NULL",
            transaction: tx());
}
