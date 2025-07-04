using Domain.Contracts.Interfaces;
using Domain.Entities;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Gets all platforms
/// </summary>
public sealed record GetPlatforms() : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(IUnitOfWork unitOfWork): IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken);
        return Result.Success(platforms ?? []);
    }
}