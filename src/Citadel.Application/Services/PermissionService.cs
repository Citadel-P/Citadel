using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Microsoft.Extensions.Caching.Memory;
using System.Security.Claims;

namespace Application.Services;

internal class PermissionService(IUnitOfWork uow, IMemoryCache memoryCache) : IPermissionService
{
    private static readonly TimeSpan ActorScopeCacheTtl = TimeSpan.FromMinutes(30);

    public Task<Result> EnforceAsync<TMessage>(TMessage message, ClaimsPrincipal user, CancellationToken cancellationToken = default) where TMessage : notnull
        => PermissionPipeline.Enforce(
            message,
            user.GetUserId(),
            this,
            cancellationToken
        );

    public async Task<bool> HasPermissionAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid? resourceId,
        CancellationToken ct)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermission == SpecificPermission.None ? null : [specificPermission]))
            throw new InvalidOperationException(
                $"Invalid runtime permission check: [{resourceType}]-[{permissionLevel}] with specific [{specificPermission}] is not an allowed combination.");

        // Per resource permissions are not cached, as they are expected to be less common and more dynamic.
        var actorIds = await GetActorScopeAsync(userId, ct);

        return await uow.Users.HasPermissionAsync(userId, resourceType, permissionLevel, specificPermission, resourceId, actorIds, ct);
    }

    public async Task<bool> HasPermissionForAllAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid[] resourceIds,
        CancellationToken ct)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermission == SpecificPermission.None ? null : [specificPermission]))
            throw new InvalidOperationException(
                $"Invalid runtime permission check: [{resourceType}]-[{permissionLevel}] with specific [{specificPermission}] is not an allowed combination.");
        
        // empty array is trivially allowed
        if (resourceIds.Length == 0)
            return true;

        var actorIds = await GetActorScopeAsync(userId, ct);
        return await uow.Users.HasPermissionForAllAsync(userId, resourceType, permissionLevel, specificPermission, resourceIds, actorIds, ct);
    }

    private async ValueTask<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken ct)
    {
        if (memoryCache.TryGetValue<Guid[]>(userId, out var cachedActorIds))
        {
            if (cachedActorIds != null && cachedActorIds.Length > 0)
                return cachedActorIds;
        }

        var actorIds = await uow.Users.GetActorScopeAsync(userId, ct);

        var cacheEntryOptions = new MemoryCacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = ActorScopeCacheTtl
        };

        memoryCache.Set(userId, actorIds, cacheEntryOptions);
        return actorIds;
    }

    private readonly record struct PermissionCacheKey(Guid UserId, ResourceType ResourceType, PermissionLevel PermissionLevel, SpecificPermission SpecificPermission);
}
