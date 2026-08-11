using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Domain.Entities.Platforms;

namespace Application.Features.Volumes.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null)
    : IQuery<Result<IEnumerable<DockerVolumeResult>>>;

internal class ListVolumesHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> connectorFactory,
    IUnitOfWork unitOfWork) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolumeResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolumeResult>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            var projections = await unitOfWork.Swarm.GetNodeVolumesAsync(query.PlatformId, cancellationToken);
            var projected = projections.Select(static value => value.Resource);
            return Result.Success<IEnumerable<DockerVolumeResult>>(ApplyFilters(projected, query));
        }

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
        var result = await volumeConnector.ListVolumesAsync(args, cancellationToken);
        if (result.IsFailure(out var volumeError, out var volumes))
            return Result.Failure<IEnumerable<DockerVolumeResult>>(volumeError);

        var list = volumes as DockerVolumeResult[] ?? [.. volumes];
        foreach (var volume in list)
            volume.PlatformId = query.PlatformId;

        return Result.Success<IEnumerable<DockerVolumeResult>>(list);
    }

    private static IEnumerable<DockerVolumeResult> ApplyFilters(
        IEnumerable<DockerVolumeResult> values,
        ListVolumes query) => values.Where(value =>
        (!query.Dangling.HasValue || query.Dangling.Value == !value.InUse)
        && (string.IsNullOrWhiteSpace(query.Driver)
            || string.Equals(value.Driver, query.Driver, StringComparison.OrdinalIgnoreCase))
        && (string.IsNullOrWhiteSpace(query.Name)
            || value.Name.Contains(query.Name, StringComparison.OrdinalIgnoreCase)));
}
