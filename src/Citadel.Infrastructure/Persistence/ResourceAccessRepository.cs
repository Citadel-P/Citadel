using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class ResourceAccessRepository(IDbConnection db, Func<IDbTransaction> tx) : IResourceAccessRepository
{
    public async Task<IEnumerable<ResourceAccessDetails>> GetAllByActorIdAsync(
    Guid actorId,
    CancellationToken cancellationToken)
    {
        const string sql = """
            WITH ResourceLookup(ResourceId, ResourceType, ResourceName) AS (
                SELECT Id, @PlatformResourceType, Name FROM Platforms
                UNION ALL
                SELECT Id, @DeploymentResourceType, Name FROM Deployments
                UNION ALL
                SELECT Id, @StackResourceType, Name FROM Stacks
                UNION ALL
                SELECT Id, @RegistryResourceType, Name FROM Registries
                UNION ALL
                SELECT Id, @GitRepositoryResourceType, Name FROM GitRepositories
            )
            SELECT
                ra.Id,
                ra.ActorId,
                ra.ResourceId,
                ra.ResourceType,
                rl.ResourceName,
                ra.PermissionLevel,
                ra.SpecificPermissions
            FROM ResourceAccesses ra
            LEFT JOIN ResourceLookup rl
                ON rl.ResourceId = ra.ResourceId
               AND rl.ResourceType = ra.ResourceType
            WHERE ra.ActorId = @ActorId
            ORDER BY ra.ResourceType, ra.ResourceId
        """;

        var rows = await db.QueryAsync<ResourceAccessDetailsDto>(
            sql,
            new
            {
                ActorId = actorId,
                PlatformResourceType = (int)ResourceType.Platform,
                DeploymentResourceType = (int)ResourceType.Deployment,
                StackResourceType = (int)ResourceType.Stack,
                RegistryResourceType = (int)ResourceType.Registry,
                GitRepositoryResourceType = (int)ResourceType.GitRepository,
            },
            transaction: tx());

        return rows.ToDetails();
    }

    public Task<int> AddAsync(ResourceAccess resourceAccess, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, PermissionLevel, SpecificPermissions)
            VALUES (@Id, @ResourceId, @ActorId, @ResourceType, @PermissionLevel, @SpecificPermissions)
            ON CONFLICT (ResourceType, ResourceId, ActorId) DO NOTHING
            """;

        return db.ExecuteAsync(sql, new
        {
            resourceAccess.Id,
            resourceAccess.ResourceId,
            resourceAccess.ActorId,
            ResourceType = (int)resourceAccess.ResourceType,
            PermissionLevel = (int)resourceAccess.PermissionLevel,
            SpecificPermissions = Permission.ToSpecificPermissionsMask(resourceAccess.SpecificPermissions),
            cancellationToken,
        }, transaction: tx());
    }

    public Task<int> RemoveAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM ResourceAccesses
            WHERE ActorId = @ActorId
              AND ResourceType = @ResourceType
              AND ResourceId = @ResourceId
              AND PermissionLevel = @PermissionLevel
              AND SpecificPermissions = @SpecificPermissions
            """;

        return db.ExecuteAsync(sql, new
        {
            ActorId = actorId,
            ResourceType = (int)resourceType,
            ResourceId = resourceId,
            PermissionLevel = (int)permissionLevel,
            SpecificPermissions = Permission.ToSpecificPermissionsMask(specificPermissions),
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
        var resourceTypes = rowsToInsert.Select(x => (int)x.ResourceType).ToArray();
        var permissionLevels = rowsToInsert.Select(x => (int)x.PermissionLevel).ToArray();
        var specificPermissions = rowsToInsert.Select(x => Permission.ToSpecificPermissionsMask(x.SpecificPermissions)).ToArray();

        const string insertSql = """
            INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, PermissionLevel, SpecificPermissions)
            SELECT src.Id, src.ResourceId, @ActorId, src.ResourceType, src.PermissionLevel, src.SpecificPermissions
            FROM unnest(@Ids::uuid[], @ResourceIds::uuid[], @ResourceTypes::integer[], @PermissionLevels::integer[], @SpecificPermissions::integer[])
                AS src(Id, ResourceId, ResourceType, PermissionLevel, SpecificPermissions)
            ON CONFLICT (ResourceType, ResourceId, ActorId) DO NOTHING
            """;

        return await db.ExecuteAsync(insertSql, new
        {
            ActorId = actorId,
            Ids = ids,
            ResourceIds = resourceIds,
            ResourceTypes = resourceTypes,
            PermissionLevels = permissionLevels,
            SpecificPermissions = specificPermissions,
            cancellationToken,
        }, transaction: tx());
    }
}
