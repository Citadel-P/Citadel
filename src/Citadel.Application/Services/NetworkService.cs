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
        var result = await networkConnector.ListNetworksAsync(args, cancellationToken);
        if (result.IsFailure(out var networkError, out var networks))
            return Result.Failure<IEnumerable<DockerNetworkResult>>(networkError);

        var list = networks as DockerNetworkResult[] ?? [.. networks];
        foreach (var network in list)
            network.PlatformId = query.PlatformId;

        return Result.Success<IEnumerable<DockerNetworkResult>>(list);
    }
}
