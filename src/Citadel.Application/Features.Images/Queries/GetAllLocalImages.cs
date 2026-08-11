using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IEnumerable<LocalImageInventoryItem>>>;

public sealed record LocalImageInventoryItem(
    Image? StandaloneImage,
    SwarmNodeImageProjection? NodeImage);

internal class GetAllLocalImagesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllLocalImages, Result<IEnumerable<LocalImageInventoryItem>>>
{
    public async ValueTask<Result<IEnumerable<LocalImageInventoryItem>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            var projections = await unitOfWork.Swarm.GetNodeImagesAsync(query.PlatformId, cancellationToken);
            return Result.Success<IEnumerable<LocalImageInventoryItem>>(
                projections.Select(static image => new LocalImageInventoryItem(null, image)));
        }

        var images = await unitOfWork.Images.GetByPlatformIdAsync(query.PlatformId, cancellationToken);
        return Result.Success<IEnumerable<LocalImageInventoryItem>>(
            images.Select(static image => new LocalImageInventoryItem(image, null)));
    }
}
