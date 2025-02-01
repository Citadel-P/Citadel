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

    ValueTask<IEnumerable<string>> GetClientsAddresses(CancellationToken cancellationToken);

    /// <summary>
    /// Delete the Grpc clients Addresses from cache
    /// </summary>
    void DeleteClientsAddresses();
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
            
            if (platformId != Guid.Empty)
            {
                var options = new MemoryCacheEntryOptions()
                {
                    AbsoluteExpiration = DateTimeOffset.UtcNow.AddHours(12),
                };
                memoryCache.Set(daemonId, platformId, options);
            }
        }

        return platformId != Guid.Empty ? platformId : null;
    }

    public async ValueTask<IEnumerable<string>> GetClientsAddresses(CancellationToken cancellationToken)
    {
        if (!memoryCache.TryGetValue("clientsAddresses", out List<string> addresses))
        {
            addresses = await dbContext.Platforms.Select(s => s.Address).ToListAsync(cancellationToken);
            
            if (addresses.Count != 0)
            {
                var options = new MemoryCacheEntryOptions()
                {
                    AbsoluteExpiration = DateTimeOffset.UtcNow.AddHours(12),
                };
                memoryCache.Set("clientsAddresses", addresses, options);
            }
        }

        return addresses;
    }

    public void DeletePlatformId(string daemonId)
        => memoryCache.Remove(daemonId);

    public void DeleteClientsAddresses()
        => memoryCache.Remove("clientsAddresses");
}
