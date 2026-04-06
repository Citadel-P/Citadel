using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Networks.Queries;

[RequirePermission(ResourceType.Platform, ResourceAction.View)]
public sealed record ListNetworks (Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Id = null, string? Name = null) : IQuery<Result<IEnumerable<DockerNetworkResult>>>;

internal class ListNetworksHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<INetworkConnector> connectorFactory) : IQueryHandler<ListNetworks, Result<IEnumerable<DockerNetworkResult>>>
{
    public async ValueTask<Result<IEnumerable<DockerNetworkResult>>> Handle(ListNetworks query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<IEnumerable<DockerNetworkResult>>(error);
        }

        var args = new ListNetworksCommand
        (
            PlatformAddress: platform.Address,
            Id: query.Id,
            Name: query.Name,
            Driver: query.Driver,
            Dangling: query.Dangling
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await networkConnector.ListNetworksAsync(args, cancellationToken);
    }
}
