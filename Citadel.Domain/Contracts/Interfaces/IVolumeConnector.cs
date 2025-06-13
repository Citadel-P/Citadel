using Domain.Contracts.Resources.Volumes;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IVolumeConnector
{
    Task<Result<IEnumerable<DockerVolume>>> ListVolumesAsync(ListVolumesCommand volumesCommand, CancellationToken cancellationToken);
    Task<Result<DockerVolume>> CreateVolumeAsync(CreateVolumeCommand createVolumeCommand, CancellationToken cancellationToken);
    Task<Result<DockerVolume>> InspectVolumeAsync(InspectVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken);
    Task<Result> DeleteVolumeAsync(DeleteVolumeCommand removeVolumeCommand, CancellationToken cancellationToken);
}
