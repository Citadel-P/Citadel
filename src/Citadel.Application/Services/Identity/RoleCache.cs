using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

public interface IRoleCache
{
    void SetRoles(Guid userId, IEnumerable<string> roles);
    string[]? GetRoles(Guid userId);
    bool IsAdmin(Guid userId);
    void RemoveRoles(Guid userId);
}

internal sealed class RoleCache(IMemoryCache memoryCache) : IRoleCache
{
    private static readonly TimeSpan RoleCacheTtl = TimeSpan.FromMinutes(30);

    private static string Key(Guid userId) => $"user-roles:{userId}";

    public void SetRoles(Guid userId, IEnumerable<string> roles)
    {
        var arr = (roles ?? []).ToArray();
        var opts = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = RoleCacheTtl };
        memoryCache.Set(Key(userId), arr, opts);
    }

    public string[]? GetRoles(Guid userId)
    {
        return memoryCache.TryGetValue<string[]>(Key(userId), out var roles) ? roles : null;
    }

    public bool IsAdmin(Guid userId)
    {
        var roles = GetRoles(userId);
        if (roles is null) return false;
        return roles.Any(r => string.Equals(r, "admin", StringComparison.OrdinalIgnoreCase));
    }

    public void RemoveRoles(Guid userId) => memoryCache.Remove(Key(userId));
}
