using Domain.Contracts.Resources.Networks;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface INetworkConnector
{
    Task<Result<IEnumerable<DockerNetwork>>> ListNetworksAsync(ListNetworksCommand listNetworksCommand, CancellationToken cancellationToken = default);
    Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default);
    Task<Result<CreateNetworkResult>> CreateNetworkAsync(CreateNetworkCommand createNetworkCommand, CancellationToken cancellationToken = default);
    Task<Result> DeleteNetworkAsync(DeleteNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default);
}
