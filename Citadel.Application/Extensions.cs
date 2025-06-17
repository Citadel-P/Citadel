using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.EntityFrameworkCore;

namespace Application;

internal static class Extensions
{
    public static IQueryable<Container> WithLastStat(this IRepository<Container> containerRepository, Guid platformId)
        => containerRepository
            .Query().AsNoTracking()
            .OrderByDescending(s => s.Created)
            .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
            .Where(s => s.PlatformId == platformId);

    internal static async Task<(string?, Guid?, PlatformConnectorType?)> GetPlatformIdAsync(this IRepository<Container> containerRepository, string containerId, CancellationToken cancellationToken)
    {
        var platform = await containerRepository
            .Query().AsNoTracking()
            .Include(s => s.Platform)
            .Where(s => s.ContainerId.StartsWith(containerId))
            .Select(s => new { s.Platform.Address, s.PlatformId, s.Platform.ConnectorType })
            .SingleOrDefaultAsync(cancellationToken);
        return (platform?.Address, platform?.PlatformId, platform?.ConnectorType);
    }
}
