using Infrastructure.EntityFramework;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Caching.Memory;

namespace Infrastructure.Services;

public interface ICacheService
{
    /// <summary>
    /// Gets the Platform id from cache
    /// </summary>
    ValueTask<Guid?> GetPlatformId(string daemonId, CancellationToken cancellationToken);

    /// <summary>
    /// Delete the Platform id from cache
    /// </summary>
    void DeletePlatformId(string daemonId);
}

internal sealed class CacheService (
    IMemoryCache memoryCache,
    ApplicationDbContext dbContext) : ICacheService
{
    /// <inheritdoc />
    public async ValueTask<Guid?> GetPlatformId(string daemonId, CancellationToken cancellationToken)
    {
        if (!memoryCache.TryGetValue(daemonId, out Guid platformId))
        {
            platformId = await dbContext.Platforms
                                    .AsNoTracking()
                                    .Include(s => s.SystemInfo)
                                    .Where(s => s.SystemInfo.DaemonId == daemonId)
                                    .Select(s => s.Id)
                                    .FirstOrDefaultAsync(cancellationToken);
            var options = new MemoryCacheEntryOptions()
            {
                  AbsoluteExpiration = DateTimeOffset.UtcNow.AddHours(12),
            };
            if (platformId != Guid.Empty)
            {
                memoryCache.Set(daemonId, platformId, options);
            }
        }

        return platformId != Guid.Empty ? platformId : null;
    }

    public void DeletePlatformId(string daemonId)
        => memoryCache.Remove(daemonId);
    
}
