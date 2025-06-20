using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IReadOnlyList<ImageResult>>>;

internal class GetAllLocalImagesHandler(IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> connectorFactory) : IQueryHandler<GetAllLocalImages, Result<IReadOnlyList<ImageResult>>>
{
    public async ValueTask<Result<IReadOnlyList<ImageResult>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address)) 
        {
            return Result.Failure<IReadOnlyList<ImageResult>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        return await connectorFactory
            .GetConnector(connectorType)
            .ListImagesAsync(address, cancellationToken: cancellationToken);
    }
}
