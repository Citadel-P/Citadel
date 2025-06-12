using Domain.Entities;
using Microsoft.EntityFrameworkCore;

namespace Infrastructure;

public static partial class Extensions
{
    public static IQueryable<Container> WithLastStat(this DbSet<Container> containersInfo, Guid platformId)
        => containersInfo
            .AsNoTracking()
            .OrderByDescending(s => s.Created)
            .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
            .Where(s => s.PlatformId == platformId);

    public static async Task<string?> GetPlatformAddress(this DbSet<Container> containersInfo, string containerId, CancellationToken cancellationToken)
        => await containersInfo
            .AsNoTracking()
            .Include(s => s.Platform)
            .Where(s => s.ContainerId.StartsWith(containerId))
            .Select(s => s.Platform.Address)
            .FirstOrDefaultAsync(cancellationToken);
}
