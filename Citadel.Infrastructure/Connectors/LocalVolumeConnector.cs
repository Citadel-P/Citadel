using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using LightResults;

namespace Infrastructure.Connectors;

internal class LocalVolumeConnector : IVolumeConnector
{
    public Task<Result<DockerVolume>> CreateVolumeAsync(CreateVolumeCommand createVolumeCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result> DeleteVolumeAsync(DeleteVolumeCommand removeVolumeCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<DockerVolume>> InspectVolumeAsync(InspectVolumeCommand inspectVolumeCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<IEnumerable<DockerVolume>>> ListVolumesAsync(ListVolumesCommand volumesCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
