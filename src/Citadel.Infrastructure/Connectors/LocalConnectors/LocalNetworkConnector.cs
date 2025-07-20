using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalNetworkConnector(INetworkService networkService) : INetworkConnector
{
    public async Task<Result<IEnumerable<DockerNetworkResult>>> ListNetworksAsync(ListNetworksCommand listNetworksCommand, CancellationToken cancellationToken = default)
    {
        var command = new Hosting.DockerClient.Models.Networks.ListNetworksCommand
        (
            Id: listNetworksCommand.Id,
            Name: listNetworksCommand.Name,
            Driver: listNetworksCommand.Driver,
            Dangling: listNetworksCommand.Dangling
        );
        var result = await networkService.ListAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, NetworkMappers.Map);
    }

    public async Task<Result<CreateDockerNetworkResult>> CreateNetworkAsync(CreateDockerNetworkCommand createNetworkCommand, CancellationToken cancellationToken = default)
    {
        var command = new Hosting.DockerClient.Models.Networks.CreateNetworkCommand
        (
            Name: createNetworkCommand.Name,
            Driver: createNetworkCommand.Driver,
            Scope: createNetworkCommand.Scope,
            Internal: createNetworkCommand.Internal,
            Attachable: createNetworkCommand.Attachable,
            Ingress: createNetworkCommand.Ingress,
            EnableIPv6: createNetworkCommand.EnableIPv6,
            EnableIPv4: createNetworkCommand.EnableIPv4,
            ConfigOnly: createNetworkCommand.ConfigOnly,
            ConfigFrom: !string.IsNullOrEmpty(createNetworkCommand.ConfigFrom?.Network) 
                            ? new Hosting.DockerClient.Models.Networks.ConfigFrom(createNetworkCommand.ConfigFrom?.Network)
                            : null,
            Ipam: createNetworkCommand.Ipam?.Map(),
            Options: createNetworkCommand.Options?.ToDictionary() ?? [],
            Labels: createNetworkCommand.Labels?.ToDictionary() ?? []
        );
        var result = await networkService.CreateAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, NetworkMappers.Map);
    }

    public async Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default)
    {
        var result = await networkService.InspectAsync(inspectNetworkCommand.NetworkId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, NetworkMappers.Map);
    }

    public async Task<Result> DeleteNetworkAsync(DeleteDockerNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default)
    {
        var command = new Hosting.DockerClient.Models.Networks.DeleteNetworkCommand
        (
           Ids: [.. deleteNetworkCommand.Ids]
        );
        var result = await networkService.DeleteAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResultForNoContent(result);
    }
}
