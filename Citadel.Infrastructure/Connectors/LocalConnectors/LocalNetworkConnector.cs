using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalNetworkConnector : INetworkConnector
{
    public Task<Result<CreateDockerNetworkResult>> CreateNetworkAsync(CreateDockerNetworkCommand createNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result> DeleteNetworkAsync(DeleteDockerNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result<IEnumerable<DockerNetworkResult>>> ListNetworksAsync(ListNetworksCommand listNetworksCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }
}
