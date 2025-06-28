using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalVolumeConnector(IVolumeService volumeService) : IVolumeConnector
{
    public async Task<Result<IEnumerable<DockerVolumeResult>>> ListVolumesAsync(ListdDockerVolumesCommand volumesCommand, CancellationToken cancellationToken)
    {
        var result = await volumeService.ListAsync(new Hosting.DockerClient.Models.Volumes.ListVolumesCommand(volumesCommand.Dangling, volumesCommand.Driver, volumesCommand.Name), cancellationToken);
        return ServiceResultHandlers.HandleResult(result, VolumeMappers.Map);
    }

    public async Task<Result<DockerVolumeResult>> CreateVolumeAsync(CreateDockerVolumeCommand createVolumeCommand, CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Volumes.CreateVolumeCommand
        (
            Name: createVolumeCommand.Name,
            Driver: createVolumeCommand.Driver,
            Labels: createVolumeCommand.Labels?.ToDictionary() ?? [],
            DriverOpts: createVolumeCommand.Options?.ToDictionary() ?? []
        );

        var result = await volumeService.CreateAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, VolumeMappers.Map);

    }

    public async Task<Result<DockerVolumeResult>> InspectVolumeAsync(InspectDockerVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
    {
        var result = await volumeService.InspectAsync(inspectVolumeCommand.Name, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, VolumeMappers.Map);
    }

    public async Task<Result> DeleteVolumeAsync(DeleteDockerVolumeCommand removeVolumeCommand, CancellationToken cancellationToken)
    {
        var command = new Hosting.DockerClient.Models.Volumes.DeleteVolumeCommand
        (
            Names: [.. removeVolumeCommand.Names],
            Force: removeVolumeCommand.Force ?? false
        );
        var result = await volumeService.DeleteAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResultForNoContent(result);
    }

}
