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

internal class ActivityEventRepository(IDbConnection db, Func<IDbTransaction> tx) : IActivityEventRepository
{
    public Task<int> AddAsync(ActivityEvent activityEvent, CancellationToken cancellationToken)
    {
        const string sql = @"
        INSERT INTO ActivityEvents (
            Id, PlatformId, ResourceId, ResourceName, ResourceType, EventType, Info, CreatedByActorId, CreatedAt
        )
        VALUES (
            @Id, @PlatformId, @ResourceId, @ResourceName, @ResourceType, @EventType, @Info, @CreatedByActorId, @CreatedAt
        )";
        return db.ExecuteAsync(sql, new
        {
            Id = activityEvent.Id.Format(),
            PlatformId = activityEvent.PlatformId.Value.Format(),
            ResourceId = activityEvent.ResourceId.Value.Format(),
            ResourceName = activityEvent.ResourceName,
            EventType = EnumFormatter<ActivityEventType>.GetValue(activityEvent.EventType),
            ResourceType = EnumFormatter<ActivityResourceType>.GetValue(activityEvent.ResourceType),
            Info = JsonSerializer.Serialize(activityEvent.Info, EventInfoJsonContext.Default.EventInfo),
            CreatedByActorId = activityEvent.CreatedByActorId.Format(),
            CreatedAt = activityEvent.CreatedAt
        },
        transaction: tx());
    }

    public async Task<ActivityEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken)
    {
        var sql = "SELECT * FROM ActivityEvents WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<ActivityEventDto>(sql, new { Id = id.Format() }, transaction: tx());
        return result?.ToDomain();
    }
}
