using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IReadOnlyList<DockerImage>>>;

internal class GetAllLocalImagesHandler(IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> connectorFactory) : IQueryHandler<GetAllLocalImages, Result<IReadOnlyList<DockerImage>>>
{
    public async ValueTask<Result<IReadOnlyList<DockerImage>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address)) 
        {
            return Result.Failure<IReadOnlyList<DockerImage>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        return await connectorFactory
            .GetConnector(connectorType)
            .ListImagesAsync(address, cancellationToken: cancellationToken);
    }
}
