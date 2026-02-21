using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
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
            Id, AlertRuleId, Type, Severity, Info, ResourceId, ResourceType, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @AlertRuleId, @Type, @Severity, @Info, @ResourceId, @ResourceType, @CreatedByActorId, @CreatedAt
        )";
        return db.ExecuteAsync(sql, new
        {
            Id = alertEvent.Id.Format(),
            AlertRuleId = alertEvent.AlertRuleId.Format(),
            Type = EnumFormatter<AlertType>.GetValue(alertEvent.Type),
            Severity = EnumFormatter<AlertSeverity>.GetValue(alertEvent.Severity),
            Info = JsonSerializer.Serialize(alertEvent.Info, AlertEventJsonContext.Default.AlertInfo),
            ResourceId = alertEvent.ResourceId?.Format(),
            ResourceType = EnumFormatter<AlertResourceType>.GetValue(alertEvent.ResourceType),
            CreatedByActorId = alertEvent.CreatedByActorId.Format(),
            CreatedAt = alertEvent.CreatedAt
        },
        transaction: tx());
    }

    public async Task<PagedResult<AlertEvent>> GetPagedAsync(
     Guid? resourceId,
     AlertType? alertType,
     AlertResourceType? resourceType,
     int page,
     int pageSize,
     CancellationToken cancellationToken)
    {
        const string SelectAlerts = """
        SELECT
            a.Id,
            a.AlertRuleId,
            a.Type,
            a.Severity,
            a.Info,
            a.ResourceId,
            a.ResourceType,
            a.CreatedByActorId,
            a.CreatedAt
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType)
        ORDER BY a.CreatedAt DESC
        LIMIT @PageSize OFFSET @Offset;
        """;

        const string CountAlerts = """
        SELECT COUNT(*)
        FROM AlertEvents a
        WHERE (@ResourceId IS NULL OR a.ResourceId = @ResourceId)
            AND (@AlertType IS NULL OR a.Type = @AlertType)
            AND (@ResourceType IS NULL OR a.ResourceType = @ResourceType);
        """;

        var offset = (page - 1) * pageSize;

        var p = new
        {
            ResourceId = resourceId?.Format(),
            AlertType = alertType is null ? null : EnumFormatter<AlertType>.GetValue(alertType.Value),
            ResourceType = resourceType is null ? null : EnumFormatter<AlertResourceType>.GetValue(resourceType.Value),
            PageSize = pageSize,
            Offset = offset,
        };

        var totalCount = await db.QuerySingleAsync<int>(
            CountAlerts, p, transaction: tx());

        var rows = await db.QueryAsync<AlertEventDto>(
            SelectAlerts, p, transaction: tx());

        return new PagedResult<AlertEvent>(
            [.. rows.Select(AlertEventMappers.ToDomain)],
            totalCount,
            page,
            pageSize);
    }
}
