using Citadel.Networks.V1;
using Citadel.SharedModels.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeNetworkConnector(IEdgeAgentCommandRouter commandRouter) : INetworkConnector
{
    public async Task<Result<IEnumerable<DockerNetworkResult>>> ListNetworksAsync(ListNetworksCommand command, CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<IEnumerable<DockerNetworkResult>>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.NetworkList,
            new ListNetworksRequest
            {
                Id = command.Id,
                Name = command.Name,
                Driver = command.Driver,
                Dangling = command.Dangling
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? ListNetworksResponse.Parser.ParseFrom(response.Payload).Map().OrderByDescending(network => network.Created).ToList()
            : Result.Failure<IEnumerable<DockerNetworkResult>>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.NetworkList, response));
    }

    public async Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(inspectNetworkCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<DockerNetworkDetails>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.NetworkInspect,
            new InspectNetworkRequest { Id = inspectNetworkCommand.NetworkId }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? InspectNetworkResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<DockerNetworkDetails>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.NetworkInspect, response));
    }

    public async Task<Result<CreateDockerNetworkResult>> CreateNetworkAsync(CreateDockerNetworkCommand command, CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<CreateDockerNetworkResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.NetworkCreate,
            new CreateNetworkRequest
            {
                Name = command.Name,
                Driver = command.Driver ?? "bridge",
                Scope = command.Scope ?? "local",
                Internal = command.Internal,
                Attachable = command.Attachable,
                Ingress = command.Ingress,
                EnableIPv6 = command.EnableIPv6,
                EnableIPv4 = command.EnableIPv4,
                ConfigOnly = command.ConfigOnly ?? false,
                ConfigFrom = new ConfigFromMessage
                {
                    Network = command.ConfigFrom?.Network ?? string.Empty
                },
                Ipam = new IPAMMessage
                {
                    Driver = command.Ipam?.Driver ?? "default",
                    Config =
                    {
                        command.Ipam?.Config?.Select(config => new IPAMConfigMessage
                        {
                            Subnet = config.Subnet ?? string.Empty,
                            IpRange = config.IpRange ?? string.Empty,
                            Gateway = config.Gateway ?? string.Empty
                        }) ?? []
                    },
                    Options = { command.Ipam?.Options.ToDictionary() ?? [] }
                },
                Labels = { command.Labels.ToDictionary() ?? [] },
                Options = { command.Options.ToDictionary() ?? [] }
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? new CreateDockerNetworkResult(CreateNetworkResponse.Parser.ParseFrom(response.Payload).Id)
            : Result.Failure<CreateDockerNetworkResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.NetworkCreate, response));
    }

    public async Task<Result> DeleteNetworkAsync(DeleteDockerNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(deleteNetworkCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.NetworkDelete,
            new DeleteNetworkRequest { Ids = { deleteNetworkCommand.Ids } }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.NetworkDelete, response));
    }
}
