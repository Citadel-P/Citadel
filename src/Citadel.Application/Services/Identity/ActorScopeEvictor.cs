using Domain.Contracts.Interfaces;

namespace Application.Services.Identity;

internal interface IActorScopeEvictor
{
    Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    Task EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
    Task EvictPermissionsForRoleAsync(Guid roleId, CancellationToken cancellationToken = default);
}

internal sealed class ActorScopeEvictor(
    IUnitOfWork unitOfWork,
    IRoleCache roleCache,
    IActorScopeProvider actorScopeProvider,
    IPermissionCache permissionCache) : IActorScopeEvictor
{
    public async Task EvictPermissionsForActorAsync(
        Guid actorId,
        CancellationToken cancellationToken = default)
    {
        var userIds = await unitOfWork.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);
        await EvictUsers(userIds, cancellationToken);
    }

    public async Task EvictPermissionsForRoleAsync(
        Guid roleId,
        CancellationToken cancellationToken = default)
    {
        var actorIds = await unitOfWork.Roles.GetActorIdsByRoleIdAsync(roleId, cancellationToken);
        var userIds = new List<Guid>();

        foreach (var actorId in actorIds)
            userIds.AddRange(await unitOfWork.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken));

        await EvictUsers(userIds, cancellationToken);
    }

    public async Task EvictUsers(
        IEnumerable<Guid> userIds,
        CancellationToken cancellationToken = default)
    {
        var ids = userIds.Distinct().ToArray();
        if (ids.Length == 0)
            return;

        foreach (var userId in ids)
        {
            roleCache.RemoveRoles(userId);
            permissionCache.InvalidateUser(userId);
        }

        await actorScopeProvider.InvalidateManyAsync(ids, cancellationToken);
    }
}
