using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null) 
    : IQuery<Result<IEnumerable<DockerVolumeResult>>>;

internal class ListVolumesHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IVolumeConnector> connectorFactory) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolumeResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolumeResult>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<IEnumerable<DockerVolumeResult>>(error);
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