using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal interface IActorScopeEvictor
{
    Task EvictForActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    void EvictUsersAsync(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
}

internal sealed class ActorScopeEvictor(IUnitOfWork uow, IMemoryCache memoryCache) : IActorScopeEvictor
{

    public async Task EvictForActorAsync(Guid actorId, CancellationToken cancellationToken = default)
    {
        var userIds = await uow.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);

        foreach (var id in (userIds ?? Enumerable.Empty<Guid>()).Distinct())
        {
            memoryCache.Remove(id);
        }
    }

    public void EvictUsersAsync(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default)
    {
        foreach (var id in userIds ?? Enumerable.Empty<Guid>())
        {
            memoryCache.Remove(id);
        }
    }
}
