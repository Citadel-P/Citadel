using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IReadOnlyList<ImageResult>>>;

internal class GetAllLocalImagesHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IImageConnector> connectorFactory) : IQueryHandler<GetAllLocalImages, Result<IReadOnlyList<ImageResult>>>
{
    public async ValueTask<Result<IReadOnlyList<ImageResult>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform))
        {
            return Result.Failure<IReadOnlyList<ImageResult>>(new NotFoundError("Platform ID not found."));
        }

        return await connectorFactory
            .GetConnector(platform.ConnectorType)
            .ListImagesAsync(platform.Address, cancellationToken: cancellationToken);
    }
}
