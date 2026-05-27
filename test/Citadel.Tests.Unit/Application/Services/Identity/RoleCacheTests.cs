using Application.Services.Identity;
using Microsoft.Extensions.Caching.Memory;

namespace Tests.Unit.Application.Services.Identity;

public class RoleCacheTests
{
    [Fact]
    public void SetAndGetRoles_ShouldStoreRolesInMemoryCache()
    {
        var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var roleCache = new RoleCache(memoryCache);

        var userId = Guid.NewGuid();
        roleCache.SetRoles(userId, new[] { "user", "admin" });

        var roles = roleCache.GetRoles(userId);

        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
        Assert.True(roleCache.IsAdmin(userId));
    }

    [Fact]
    public void RemoveRoles_ShouldRemoveRolesFromCache()
    {
        var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var roleCache = new RoleCache(memoryCache);

        var userId = Guid.NewGuid();
        roleCache.SetRoles(userId, new[] { "user" });
        Assert.False(roleCache.IsAdmin(userId));

        roleCache.RemoveRoles(userId);

        var roles = roleCache.GetRoles(userId);
        Assert.Null(roles);
        Assert.False(roleCache.IsAdmin(userId));
    }
}
