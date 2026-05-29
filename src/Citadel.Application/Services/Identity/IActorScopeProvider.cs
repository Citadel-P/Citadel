namespace Application.Services.Identity;

public interface IActorScopeProvider
{
    Task<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken cancellationToken = default);

    bool TryGetCachedActorScope(Guid userId, out Guid[]? actorIds);

    Task InvalidateAsync(Guid userId, CancellationToken cancellationToken = default);

    Task InvalidateManyAsync(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
}
