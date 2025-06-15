using Domain;
using Domain.Entities;
using Microsoft.EntityFrameworkCore;

namespace Application;

internal static class Extensions
{
    public static IQueryable<Container> WithLastStat(this DbSet<Container> containersInfo, Guid platformId)
        => containersInfo
            .AsNoTracking()
            .OrderByDescending(s => s.Created)
            .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
            .Where(s => s.PlatformId == platformId);

    public static async Task<(string?, Guid?, PlatformConnectorType?)> GetPlatformIdAsync(this DbSet<Container> containersInfo, string containerId, CancellationToken cancellationToken)
    {
        var platform = await containersInfo
            .AsNoTracking()
            .Include(s => s.Platform)
            .Where(s => s.ContainerId.StartsWith(containerId))
            .Select(s => new { s.Platform.Address, s.PlatformId, s.Platform.ConnectorType })
            .SingleOrDefaultAsync(cancellationToken);
        return (platform?.Address, platform?.PlatformId, platform?.ConnectorType);
    }
}
