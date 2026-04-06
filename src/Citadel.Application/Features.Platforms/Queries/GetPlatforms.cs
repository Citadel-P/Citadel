using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, ResourceAction.View)]
public sealed record GetPlatforms() : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(IUnitOfWork unitOfWork): IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken);
        return Result.Success(platforms ?? []);
    }
}