using Domain.Contracts.Interfaces;
using Domain.Entities;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Gets all platforms
/// </summary>
public sealed record GetPlatforms() : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(IUnitOfWork unitOfWork): IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        // Only include the most recent stat for the platform
        var platforms = await unitOfWork.Platforms
                .Query().AsNoTracking()
                .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
                .OrderBy(s => s.Name)
                .ToListAsync(cancellationToken);

        return platforms;
    }
}