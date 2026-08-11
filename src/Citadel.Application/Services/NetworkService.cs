using Application.Features.Networks.Queries;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using LightResults;
using Domain.Entities.Platforms;

namespace Application.Services;

internal interface INetworkService
{
    Task<Result<IEnumerable<DockerNetworkResult>>> List(ListNetworks query, CancellationToken cancellationToken);
}

internal class NetworkService(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<INetworkConnector> connectorFactory,
    IUnitOfWork unitOfWork) : INetworkService
{
    public async Task<Result<IEnumerable<DockerNetworkResult>>> List(ListNetworks query, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            var cluster = await unitOfWork.Swarm.GetNetworksAsync(query.PlatformId, cancellationToken);
            var local = await unitOfWork.Swarm.GetNodeNetworksAsync(query.PlatformId, cancellationToken);
            var projectedNetworks = cluster.Select(value => MapClusterNetwork(query.PlatformId, value))
                .Concat(local.Select(static value => value.Resource));
            return Result.Success<IEnumerable<DockerNetworkResult>>(ApplyFilters(projectedNetworks, query));
        }

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

    private static DockerNetworkResult MapClusterNetwork(Guid platformId, SwarmNetworkProjection value)
    {
        var network = new DockerNetworkResult(
            value.Name,
            value.DockerNetworkId,
            value.DockerCreatedAt?.ToString("O") ?? string.Empty,
            value.Driver,
            value.Scope,
            EnableIPv4: value.Subnets.Any(static subnet => !subnet.Contains(':')),
            value.EnableIPv6,
            value.IsInternal,
            value.IsAttachable,
            value.IsIngress,
            ConfigOnly: false,
            InUse: value.ServiceNames.Count > 0,
            ConfigFrom: null,
            new IpAddressManagementConfig(
                Driver: "default",
                value.Subnets.Select(static subnet => new IpamSubnetConfiguration(subnet, null, null)).ToArray(),
                Options: new Dictionary<string, string>()),
            Options: new Dictionary<string, string>(),
            value.Labels)
        {
            PlatformId = platformId,
            IsStale = value.IsStale
        };
        return network;
    }

    private static IEnumerable<DockerNetworkResult> ApplyFilters(
        IEnumerable<DockerNetworkResult> values,
        ListNetworks query) => values.Where(value =>
        (!query.Dangling.HasValue || query.Dangling.Value == !value.InUse)
        && (string.IsNullOrWhiteSpace(query.Driver)
            || string.Equals(value.Driver, query.Driver, StringComparison.OrdinalIgnoreCase))
        && (string.IsNullOrWhiteSpace(query.Id)
            || value.Id.StartsWith(query.Id, StringComparison.OrdinalIgnoreCase))
        && (string.IsNullOrWhiteSpace(query.Name)
            || value.Name.Contains(query.Name, StringComparison.OrdinalIgnoreCase)));
}
