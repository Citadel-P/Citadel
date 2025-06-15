using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using LightResults;

namespace Infrastructure.Connectors;

internal class LocalNetworkConnector : INetworkConnector
{
    public Task<Result<CreateNetworkResult>> CreateNetworkAsync(CreateNetworkCommand createNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result> DeleteNetworkAsync(DeleteNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    public Task<Result<IEnumerable<DockerNetwork>>> ListNetworksAsync(ListNetworksCommand listNetworksCommand, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }
}
