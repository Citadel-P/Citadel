using Domain.Contracts.Resources.Networks;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing container networks.
/// </summary>
public interface INetworkConnector
{
    Task<Result<IEnumerable<DockerNetworkResult>>> ListNetworksAsync(ListNetworksCommand listNetworksCommand, CancellationToken cancellationToken = default);
    Task<Result<DockerNetworkDetails>> InspectNetworkAsync(InspectNetworkCommand inspectNetworkCommand, CancellationToken cancellationToken = default);
    Task<Result<CreateDockerNetworkResult>> CreateNetworkAsync(CreateDockerNetworkCommand createNetworkCommand, CancellationToken cancellationToken = default);
    Task<Result> DeleteNetworkAsync(DeleteDockerNetworkCommand deleteNetworkCommand, CancellationToken cancellationToken = default);
}
