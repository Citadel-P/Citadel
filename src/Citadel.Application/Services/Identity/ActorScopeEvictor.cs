using Domain.Contracts.Interfaces;
using Application.Services.SignalR;

namespace Application.Services.Identity;

internal interface IActorScopeEvictor
{
    Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    Task EvictActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    Task EvictActors(IEnumerable<Guid> actorIds, CancellationToken cancellationToken = default);
    Task EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
    Task EvictPermissionsForRoleAsync(Guid roleId, CancellationToken cancellationToken = default);
}

internal sealed class ActorScopeEvictor(
    IUnitOfWork unitOfWork,
    IRoleCache roleCache,
    IActorScopeProvider actorScopeProvider,
    IPermissionCache permissionCache,
    IUserConnectionRevoker connectionRevoker) : IActorScopeEvictor
{
    public async Task EvictActorAsync(Guid actorId, CancellationToken cancellationToken = default)
        => await EvictActors([actorId], cancellationToken);

    public async Task EvictActors(
        IEnumerable<Guid> actorIds,
        CancellationToken cancellationToken = default)
    {
        var ids = actorIds.Distinct().ToArray();
        if (ids.Length == 0)
            return;
        EvictCaches(ids);
        var userIds = await unitOfWork.Users.GetUserIdsByActorIdsAsync(ids, cancellationToken);
        connectionRevoker.RevokeUsers(userIds);
        await actorScopeProvider.InvalidateManyAsync(ids, cancellationToken);
    }

    public async Task EvictPermissionsForActorAsync(
        Guid actorId,
        CancellationToken cancellationToken = default)
    {
        var actorIds = await unitOfWork.Teams.GetAffectedPrincipalActorIdsAsync(actorId, cancellationToken);
        await EvictActors(actorIds, cancellationToken);
    }

    public async Task EvictPermissionsForRoleAsync(
        Guid roleId,
        CancellationToken cancellationToken = default)
    {
        var actorIds = await unitOfWork.Roles.GetActorIdsByRoleIdAsync(roleId, cancellationToken);
        var affectedActorIds = new List<Guid>();

        foreach (var actorId in actorIds)
            affectedActorIds.AddRange(await unitOfWork.Teams.GetAffectedPrincipalActorIdsAsync(actorId, cancellationToken));

        await EvictActors(affectedActorIds, cancellationToken);
    }

    public async Task EvictUsers(
        IEnumerable<Guid> userIds,
        CancellationToken cancellationToken = default)
    {
        var ids = userIds.Distinct().ToArray();
        if (ids.Length == 0)
            return;

        var actorIds = (await unitOfWork.Users.GetActorIdsAsync(ids, cancellationToken)).Distinct().ToArray();
        if (actorIds.Length == 0)
            return;

        EvictCaches(actorIds);
        connectionRevoker.RevokeUsers(ids);
        await actorScopeProvider.InvalidateManyAsync(actorIds, cancellationToken);
    }

    private void EvictCaches(IEnumerable<Guid> actorIds)
    {
        foreach (var actorId in actorIds)
        {
            roleCache.RemoveRoles(actorId);
            permissionCache.InvalidateActor(actorId);
        }
    }
}
