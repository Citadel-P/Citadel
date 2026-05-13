using Application.Features.Networks.Queries;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using LightResults;

namespace Application.Services;

internal interface INetworkService
{
    Task<Result<IEnumerable<DockerNetworkResult>>> List(ListNetworks query, CancellationToken cancellationToken);
}

internal class NetworkService(IPlatformContainerCache platformContainerCache, IConnectorFactory<INetworkConnector> connectorFactory) : INetworkService
{
    public async Task<Result<IEnumerable<DockerNetworkResult>>> List(ListNetworks query, CancellationToken cancellationToken)
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
