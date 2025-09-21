using Microsoft.Extensions.Caching.Memory;

namespace Application.Services;

public interface IRegistryCache
{
    void Set(Guid platformId, string imageId, Guid registryId);
    bool TryGet(Guid platformId, string imageId, out Guid registryId);
}

internal sealed class RegistryCache(IMemoryCache memoryCache) : IRegistryCache
{
    private readonly IMemoryCache memoryCache = memoryCache;

    public void Set(Guid platformId, string imageId, Guid registryId)
    {
        memoryCache.Set((platformId, imageId), registryId, TimeSpan.FromMinutes(5));
    }

    public bool TryGet(Guid platformId, string imageId, out Guid registryId)
    {
        if (memoryCache.TryGetValue((platformId, imageId), out Guid value))
        {
            registryId = value;
            return true;
        }

        registryId = Guid.Empty;
        return false;
    }
}