using Domain.Contracts.Resources.Swarm;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface ISwarmConnector
{
    Task<Result<IReadOnlyList<SwarmNodeResult>>> ListNodesAsync(
        ListSwarmNodesCommand command,
        CancellationToken cancellationToken = default);

    Task<Result<SwarmNodeResult>> InspectNodeAsync(
        InspectSwarmNodeCommand command,
        CancellationToken cancellationToken = default);

    Task<Result<IReadOnlyList<SwarmServiceResult>>> ListServicesAsync(
        ListSwarmServicesCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmServiceResult>> InspectServiceAsync(
        InspectSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(
        ListSwarmTasksCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmTaskResult>> InspectTaskAsync(
        InspectSwarmTaskCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmNetworkResult>>> ListNetworksAsync(
        ListSwarmNetworksCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmNetworkResult>> InspectNetworkAsync(
        InspectSwarmNetworkCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmSecretResult>>> ListSecretsAsync(
        ListSwarmSecretsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmSecretResult>> InspectSecretAsync(
        InspectSwarmSecretCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(
        ListSwarmConfigsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmConfigResult>> InspectConfigAsync(
        InspectSwarmConfigCommand command,
        CancellationToken cancellationToken = default);
}
