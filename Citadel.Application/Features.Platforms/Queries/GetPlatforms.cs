using Infrastructure.Entities;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Gets all platforms
/// </summary>
public sealed record GetPlatforms() : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(ApplicationDbContext dbContext)
    : IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var platforms = await dbContext.Platforms
                                        .AsNoTracking()
                                        .AsSplitQuery()
                                        .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1)) // We only care about the last record)
                                        .Include(s => s.SystemInfo)
                                        .ThenInclude(s => s.SwarmInfo)
                                        .ThenInclude(s => s.RemoteManagers)
                                        .OrderBy(s => s.Name)
                                        .ToListAsync(cancellationToken);

        return Result.Success<IEnumerable<Platform>>(platforms);
    }
}