using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Microsoft.Extensions.Caching.Memory;
using System.Security.Claims;

namespace Application.Services;

internal class PermissionService(IUnitOfWork uow, IMemoryCache memoryCache) : IPermissionService
{
    private static readonly TimeSpan PermissionCacheTtl = TimeSpan.FromSeconds(60);

    public  Task<Result> EnforceAsync<TMessage>(TMessage message, ClaimsPrincipal user, CancellationToken cancellationToken = default) where TMessage : notnull
        => PermissionPipeline.Enforce(
            message,
            user.GetUserId(),
            this,
            cancellationToken
        );
    
    public Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct)
    {
        // Per resource permissions are not cached, as they are expected to be less common and more dynamic.
        if (resourceId is not null)
        {
            return uow.Users.HasPermissionAsync(userId, resourceType, action, resourceId, ct);
        }

        // For global permissions (eg list deployments), we cache the result to reduce database load and improve performance. 
        var cacheKey = new PermissionCacheKey(userId, resourceType, action);
        return memoryCache.GetOrCreateAsync(cacheKey, async entry =>
        {
            entry.SlidingExpiration = PermissionCacheTtl;
            return await uow.Users.HasPermissionAsync(userId, resourceType, action, null, ct);
        })!;
    }

    private readonly record struct PermissionCacheKey(Guid UserId, ResourceType ResourceType, ResourceAction Action);
}
