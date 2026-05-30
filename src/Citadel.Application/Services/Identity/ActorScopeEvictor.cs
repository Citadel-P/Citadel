using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal interface IActorScopeEvictor
{
    Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    Task EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
    Task EvictPermissionsForRoleAsync(Guid roleId, CancellationToken cancellationToken = default);
}

internal sealed class ActorScopeEvictor(IUnitOfWork uow, IMemoryCache memoryCache, IRoleCache roleCache, IActorScopeProvider actorScopeProvider, IPermissionCache permissionCache) : IActorScopeEvictor
{
    public async Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default)
    {
        var userIds = await uow.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);
        var distinctUserIds = (userIds ?? []).Distinct().ToArray();

        // Evict actor-scope keys and permission index entries first
        foreach (var id in distinctUserIds)
        {
            // Invalidate permission entries for the user via the permission cache abstraction
            permissionCache.InvalidateUser(id);
            // Keep actor-scope removal via memoryCache for now (actor scope key owned by ActorScopeProvider)
            memoryCache.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(id));
        }

        // Also remove cached actor scope entries from centralized provider
        await actorScopeProvider.InvalidateManyAsync(distinctUserIds, cancellationToken);

        // Refresh roles for the users that remain (batch)
        if (distinctUserIds.Length > 0)
            await RefreshUserRolesBatchForActorAsync(actorId, distinctUserIds, cancellationToken);
    }

    public async Task EvictPermissionsForRoleAsync(Guid roleId, CancellationToken cancellationToken = default)
    {
        var actorIds = (await uow.Roles.GetActorIdsByRoleIdAsync(roleId, cancellationToken)) ?? [];

        if (!actorIds.Any()) return;

        var userIds = new List<Guid>();
        foreach (var actorId in actorIds)
        {
            var uids = await uow.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);
            if (uids is not null)
                userIds.AddRange(uids);
        }

        var distinctUserIds = userIds.Distinct().ToArray();
        if (distinctUserIds.Length == 0) return;

        var idArray = distinctUserIds.Distinct().ToArray();
        if (idArray.Length == 0) return;

        // Evict actor-scope keys and permission index entries up-front
        foreach (var id in idArray)
        {
            permissionCache.InvalidateUser(id);
            memoryCache.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(id));
        }

        // Also remove cached actor scope entries
        await actorScopeProvider.InvalidateManyAsync(idArray, cancellationToken);

        // Batch fetch users to determine which exist and their actor ids
        var existingUsers = (await uow.Users.GetAllAsync(idArray, cancellationToken)) ?? [];

        // Build a hashset of existing ids to avoid extra allocations when checking missing users
        var existingUserIdsSet = existingUsers.Select(static u => u.Id).ToHashSet();

        // Remove role cache for users that no longer exist
        foreach (var userId in idArray)
        {
            if (!existingUserIdsSet.Contains(userId))
                roleCache.RemoveRoles(userId);
        }

        // Group existing users by actor and refresh roles per actor
        var groupByActor = existingUsers.GroupBy(u => u.ActorId).ToArray();
        if (groupByActor.Length > 0)
        {
            var actorIds2 = groupByActor.Select(g => g.Key).ToArray();
            var actorRoleMap = await uow.Roles.GetActorRoleIdsAsync(actorIds2, cancellationToken) ?? new Dictionary<Guid, Guid[]>();

            // Collect all distinct role ids across all actors and fetch roles once
            var allRoleIds = actorRoleMap.Values.SelectMany(static x => x).Distinct().ToArray();
            var allRoles = allRoleIds.Length > 0
                ? (await uow.Roles.GetAllAsync(allRoleIds, cancellationToken)) ?? []
                : [];

            var roleNameMap = allRoles.ToDictionary(r => r.Id, r => r.Name);

            foreach (var group in groupByActor)
            {
                var actorId = group.Key;
                var groupUsers = group.ToArray();

                actorRoleMap.TryGetValue(actorId, out var actorRoleIds);
                actorRoleIds ??= [];

                if (actorRoleIds.Length == 0)
                {
                    // No roles for this actor -> remove roles for all users in this actor group
                    foreach (var user in groupUsers)
                        roleCache.RemoveRoles(user.Id);

                    continue;
                }

                // Map role ids to names using the pre-fetched map
                var roleNames = actorRoleIds
                    .Where(id => roleNameMap.ContainsKey(id))
                    .Select(id => roleNameMap[id])
                    .ToArray();

                if (roleNames.Length == 0)
                {
                    // No matching roles found -> remove roles for each user
                    foreach (var user in groupUsers)
                        roleCache.RemoveRoles(user.Id);
                }
                else
                {
                    // Set roles for each user in this actor group
                    foreach (var user in groupUsers)
                        roleCache.SetRoles(user.Id, roleNames);
                }
            }
        }
    }

    public async Task EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default)
    {
        var idArray = (userIds ?? []).Distinct().ToArray();
        if (idArray.Length == 0) return;

        // Evict actor-scope keys and permission index entries up-front
        foreach (var id in idArray)
        {
            permissionCache.InvalidateUser(id);
            memoryCache.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(id));
        }

        // Also remove cached actor scope entries
        await actorScopeProvider.InvalidateManyAsync(idArray, cancellationToken);

        // Batch fetch users to determine which exist and their actor ids
        var existingUsers = (await uow.Users.GetAllAsync(idArray, cancellationToken)) ?? [];

        // Build a hashset of existing ids to avoid extra allocations when checking missing users
        var existingUserIdsSet = existingUsers.Select(static u => u.Id).ToHashSet();

        // Remove role cache for users that no longer exist
        foreach (var userId in idArray)
        {
            if (!existingUserIdsSet.Contains(userId))
                roleCache.RemoveRoles(userId);
        }

        // Group existing users by actor and refresh roles per actor
        var groupByActor = existingUsers.GroupBy(u => u.ActorId).ToArray();
        if (groupByActor.Length > 0)
        {
            var actorIds = groupByActor.Select(g => g.Key).ToArray();
            var actorRoleMap = await uow.Roles.GetActorRoleIdsAsync(actorIds, cancellationToken) ?? new Dictionary<Guid, Guid[]>();

            // Collect all distinct role ids across all actors and fetch roles once
            var allRoleIds = actorRoleMap.Values.SelectMany(static x => x).Distinct().ToArray();
            var allRoles = allRoleIds.Length > 0
                ? (await uow.Roles.GetAllAsync(allRoleIds, cancellationToken)) ?? []
                : [];

            var roleNameMap = allRoles.ToDictionary(r => r.Id, r => r.Name);

            foreach (var group in groupByActor)
            {
                var actorId = group.Key;
                var groupUsers = group.ToArray();

                actorRoleMap.TryGetValue(actorId, out var actorRoleIds);
                actorRoleIds ??= [];

                if (actorRoleIds.Length == 0)
                {
                    // No roles for this actor -> remove roles for all users in this actor group
                    foreach (var user in groupUsers)
                        roleCache.RemoveRoles(user.Id);

                    continue;
                }

                // Map role ids to names using the pre-fetched map
                var roleNames = actorRoleIds
                    .Where(id => roleNameMap.ContainsKey(id))
                    .Select(id => roleNameMap[id])
                    .ToArray();

                if (roleNames.Length == 0)
                {
                    // No matching roles found -> remove roles for each user
                    foreach (var user in groupUsers)
                        roleCache.RemoveRoles(user.Id);
                }
                else
                {
                    // Set roles for each user in this actor group
                    foreach (var user in groupUsers)
                        roleCache.SetRoles(user.Id, roleNames);
                }
            }
        }
    }

    private async Task RefreshUserRolesBatchForActorAsync(Guid actorId, Guid[] userIds, CancellationToken cancellationToken)
    {
        // Determine existing users among the provided ids
        var existingUsers = (await uow.Users.GetAllAsync(userIds, cancellationToken)) ?? [];

        var existingUserIdsSet = existingUsers.Select(static u => u.Id).ToHashSet();

        // Users that no longer exist -> remove roles
        foreach (var id in userIds)
        {
            if (!existingUserIdsSet.Contains(id))
                roleCache.RemoveRoles(id);
        }

        // For actor role changes we evict role cache entries so callers will repopulate
        // on next request. This avoids silently overwriting an in-memory cache during
        // a role assignment/removal operation and ensures a fresh permission resolution.
        foreach (var u in existingUserIdsSet)
            roleCache.RemoveRoles(u);
    }

    private void EvictUserPermissionCache(Guid userId)
    {
        memoryCache.Remove(Hosting.Common.Constants.CacheKeys.ActorScope(userId));

        // Moved to IPermissionCache
        permissionCache.InvalidateUser(userId);
    }
}
