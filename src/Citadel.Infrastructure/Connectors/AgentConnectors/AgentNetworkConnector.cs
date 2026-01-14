using Citadel.Networks.V1;
using Citadel.SharedModels.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentNetworkConnector(IGrpcClientFactory clientFactory) : INetworkConnector
{
    public async Task<Result<IEnumerable<DockerNetworkResult>>> ListNetworksAsync(ListNetworksCommand command, CancellationToken cancellationToken = default)
    {
        try
        {
            var args = new ListNetworksRequest
            {
                Id = command.Id,
                Name = command.Name,
                Driver = command.Driver,
                Dangling = command.Dangling,
            };
            var client = clientFactory.GetNetworkClient(command.PlatformAddress);
            var response = await client.ListAsync(args, cancellationToken: cancellationToken);
            var networks = response.Map();

            return networks.OrderByDescending(c => c.Created).ToList();
        }
        catch (RpcException ex)
        {
            return Result.Failure<IEnumerable<DockerNetworkResult>>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default)
    {
        try
        {
            var request = new InspectNetworkRequest
            {
                Id = inspectNetworkCommand.NetworkId
            };
            var client = clientFactory.GetNetworkClient(inspectNetworkCommand.PlatformAddress);
            var result = await client.InspectAsync(request, cancellationToken: cancellationToken);

            return result.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<DockerNetworkDetails>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<CreateDockerNetworkResult>> CreateNetworkAsync(CreateDockerNetworkCommand command, CancellationToken cancellationToken = default)
    {
        try
        {
            var client = clientFactory.GetNetworkClient(command.PlatformAddress);
            var request = new CreateNetworkRequest
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
                Ipam = new IPAMMessage
                {
                    Driver = command.Ipam?.Driver ?? "default",
                    Config =
                    {
                        command.Ipam?.Config?.Select(c => new IPAMConfigMessage
                        {
                            Subnet = c.Subnet ?? "",
                            IpRange = c.IpRange ?? "",
                            Gateway = c.Gateway ?? ""
                        }) ?? []
                    },
                    Options =
                    {
                        command.Ipam?.Options.ToDictionary() ?? []
                    }
                },
                ConfigFrom = new ConfigFromMessage
                {
                    Network = command.ConfigFrom?.Network ?? ""
                },
                Labels = { command.Labels.ToDictionary() ?? [] },
                Options = { command.Options.ToDictionary() ?? [] }
            };
            var response = await client.CreateAsync(request, cancellationToken: cancellationToken);
            return new CreateDockerNetworkResult(NetworkId: response.Id);
        }
        catch (RpcException ex)
        {
            return Result.Failure<CreateDockerNetworkResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result> DeleteNetworkAsync(DeleteDockerNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default)
    {
        try
        {
            var client = clientFactory.GetNetworkClient(deleteNetworkCommand.PlatformAddress);
            await client.DeleteAsync(new DeleteNetworkRequest { Ids = { deleteNetworkCommand.Ids } }, cancellationToken: cancellationToken);
            return Result.Success();
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
