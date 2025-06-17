using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null) 
    : IQuery<Result<IEnumerable<DockerVolume>>>;

internal class ListVolumesHandler(IUnitOfWork unitOfWork, IConnectorFactory<IVolumeConnector> connectorFactory) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolume>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolume>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure<IEnumerable<DockerVolume>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListVolumesCommand
            (
                PlatformAddress: address,
                Dangling: query.Dangling,
                Driver: query.Driver,
                Name: query.Name
            );

        var volumeConnector = connectorFactory.GetConnector(connectorType);
        return await volumeConnector.ListVolumesAsync(args, cancellationToken);
    }
}