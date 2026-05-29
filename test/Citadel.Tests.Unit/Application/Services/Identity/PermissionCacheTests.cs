using System;
using System.Linq;
using Application.Services.Identity;
using Application.Permissions;
using Hosting.Common.Attributes;
using Hosting.Common;
using Microsoft.Extensions.Caching.Memory;

namespace Tests.Unit.Application.Services.Identity;

public class PermissionCacheTests
{
    [Fact]
    public async Task AddManyToIndex_And_InvalidateUser_RemovesKeys()
    {
        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var cache = new PermissionCache(memoryCache);

        var userId = Guid.NewGuid();
        var key1 = new PermissionCacheKey(userId, ResourceType.Deployment, Guid.NewGuid());
        var key2 = new PermissionCacheKey(userId, ResourceType.Deployment, Guid.NewGuid());

        cache.Set(key1, new PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));
        cache.Set(key2, new PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));

        var idxBefore = cache.GetIndex(userId);
        Assert.Contains(key1, idxBefore);
        Assert.Contains(key2, idxBefore);

        cache.InvalidateUser(userId);

        var idxAfter = cache.GetIndex(userId);
        Assert.Empty(idxAfter);
        Assert.Null(cache.Get(key1));
        Assert.Null(cache.Get(key2));
    }
}
