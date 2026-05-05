using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class ResourceAccessRepository(IDbConnection db, Func<IDbTransaction> tx) : IResourceAccessRepository
{
    public Task<int> AddAsync(ResourceAccess resourceAccess, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, Action)
            VALUES (@Id, @ResourceId, @ActorId, @ResourceType, @Action)
            ON CONFLICT (ResourceType, ResourceId, ActorId, Action) DO NOTHING
            """;

        return db.ExecuteAsync(sql, new
        {
            resourceAccess.Id,
            resourceAccess.ResourceId,
            resourceAccess.ActorId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceAccess.ResourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(resourceAccess.Action),
            cancellationToken,
        }, transaction: tx());
    }

    public Task<int> RemoveAsync(Guid actorId, ResourceType resourceType, Guid resourceId, ResourceAction action, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM ResourceAccesses
            WHERE ActorId = @ActorId
              AND ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND Action = @Action
            """;

        return db.ExecuteAsync(sql, new
        {
            ActorId = actorId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            ResourceId = resourceId,
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            cancellationToken,
        }, transaction: tx());
    }

    public async Task<int> ReplaceAsync(Guid actorId, IEnumerable<ResourceAccess> resourceAccesses, CancellationToken cancellationToken)
    {
        const string deleteSql = "DELETE FROM ResourceAccesses WHERE ActorId = @ActorId";
        await db.ExecuteAsync(deleteSql, new { ActorId = actorId, cancellationToken }, transaction: tx());

        var rowsToInsert = resourceAccesses as ResourceAccess[] ?? [.. resourceAccesses];
        if (rowsToInsert.Length == 0)
            return 0;

        var ids = rowsToInsert.Select(x => x.Id).ToArray();
        var resourceIds = rowsToInsert.Select(x => x.ResourceId).ToArray();
        var resourceTypes = rowsToInsert.Select(x => EnumFormatter<ResourceType>.GetValue(x.ResourceType)).ToArray();
        var actions = rowsToInsert.Select(x => EnumFormatter<ResourceAction>.GetValue(x.Action)).ToArray();

        const string insertSql = """
            INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, Action)
            SELECT src.Id, src.ResourceId, @ActorId, src.ResourceType, src.Action
            FROM unnest(@Ids::uuid[], @ResourceIds::uuid[], @ResourceTypes::text[], @Actions::text[])
                AS src(Id, ResourceId, ResourceType, Action)
            ON CONFLICT (ResourceType, ResourceId, ActorId, Action) DO NOTHING
            """;

        return await db.ExecuteAsync(insertSql, new
        {
            ActorId = actorId,
            Ids = ids,
            ResourceIds = resourceIds,
            ResourceTypes = resourceTypes,
            Actions = actions,
            cancellationToken,
        }, transaction: tx());
    }
}
