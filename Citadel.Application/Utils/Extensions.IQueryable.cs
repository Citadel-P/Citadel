using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;

namespace Application.Utils;

internal static partial class Extensions
{
    internal static IQueryable<ContainerInfo> WithLastStat(this DbSet<ContainerInfo> containersInfo, Guid platformId)
        => containersInfo
            .AsNoTracking()
            .OrderByDescending(s => s.Created)
            .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
            .Where(s => s.PlatformId == platformId);
}
