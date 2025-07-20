using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null) 
    : IQuery<Result<IEnumerable<DockerVolumeResult>>>;

internal class ListVolumesHandler(IUnitOfWork unitOfWork, IConnectorFactory<IVolumeConnector> connectorFactory) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolumeResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolumeResult>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<IEnumerable<DockerVolumeResult>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListdDockerVolumesCommand
            (
                PlatformAddress: platform.Address,
                Dangling: query.Dangling,
                Driver: query.Driver,
                Name: query.Name
            );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await volumeConnector.ListVolumesAsync(args, cancellationToken);
    }
}