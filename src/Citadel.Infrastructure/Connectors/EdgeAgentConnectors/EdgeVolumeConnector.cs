using Citadel.Volumes.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeVolumeConnector(IEdgeAgentCommandRouter commandRouter) : IVolumeConnector
{
    public async Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(ListdDockerVolumesCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<IEnumerable<DockerVolumeResult>>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.VolumeList,
            new ListVolumesRequest
            {
                Driver = command.Driver,
                Name = command.Name,
                Dangling = command.Dangling
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? ListVolumesResponse.Parser.ParseFrom(response.Payload).Map().OrderByDescending(volume => volume.CreatedAt).ToList()
            : Result.Failure<IEnumerable<DockerVolumeResult>>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.VolumeList, response));
    }

    public async Task<Result<DockerVolumeResult>> CreateVolumeAsync(CreateDockerVolumeCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<DockerVolumeResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.VolumeCreate,
            new CreateVolumeRequest
            {
                Name = command.Name,
                Driver = command.Driver,
                Labels = { command.Labels?.ToDictionary() ?? [] },
                Options = { command.Options?.ToDictionary() ?? [] }
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? Citadel.SharedModels.V1.VolumeResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<DockerVolumeResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.VolumeCreate, response));
    }

    public async Task<Result<DockerVolumeResult>> InspectVolumeAsync(InspectDockerVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(inspectVolumeCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<DockerVolumeResult>(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.VolumeInspect,
            new InspectVolumeRequest { Name = inspectVolumeCommand.Name }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? Citadel.SharedModels.V1.VolumeResponse.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<DockerVolumeResult>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.VolumeInspect, response));
    }

    public async Task<Result> DeleteVolumeAsync(DeleteDockerVolumeCommand command, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure(addressError!);
        }

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.VolumeDelete,
            new RemoveVolumeRequest
            {
                Names = { command.Names },
                Force = command.Force ?? false
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.VolumeDelete, response));
    }
}
