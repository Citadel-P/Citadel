using Citadel.Volumes.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentVolumeConnector(IGrpcClientFactory clientFactory) : IVolumeConnector
{
    public async Task<Result<DockerVolumeResult>> CreateVolumeAsync(CreateDockerVolumeCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetVolumeClient(command.PlatformAddress);
            var request = new CreateVolumeRequest
            {
                Name = command.Name,
                Driver = command.Driver,
                Labels = { command.Labels?.ToDictionary() ?? [] },
                Options = { command.Options?.ToDictionary() ?? [] }
            };

            var volume = await client.CreateAsync(request, cancellationToken: cancellationToken);
            return volume.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<DockerVolumeResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<DockerVolumeResult>> InspectVolumeAsync(InspectDockerVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
    {
        try
        {
            var request = new InspectVolumeRequest
            {
                Name = inspectVolumeCommand.Name
            };
            var client = clientFactory.GetVolumeClient(inspectVolumeCommand.PlatformAddress);
            var volume = await client.InspectAsync(request, cancellationToken: cancellationToken);
            return volume.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<DockerVolumeResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(ListdDockerVolumesCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var request = new ListVolumesRequest
            {
                Driver = command.Driver,
                Name = command.Name,
                Dangling = command.Dangling
            };
            var client = clientFactory.GetVolumeClient(command.PlatformAddress);
            var result = await client.ListAsync(request, cancellationToken: cancellationToken);

            return result.Map().OrderByDescending(s => s.CreatedAt).ToList();
        }
        catch (RpcException ex)
        {
            return Result.Failure<IEnumerable<DockerVolumeResult>>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result> DeleteVolumeAsync(DeleteDockerVolumeCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetVolumeClient(command.PlatformAddress);
            var request = new RemoveVolumeRequest
            {
                Names = { command.Names },
                Force = command.Force ?? false
            };
            await client.RemoveAsync(request, cancellationToken: cancellationToken);
            return Result.Success();
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
