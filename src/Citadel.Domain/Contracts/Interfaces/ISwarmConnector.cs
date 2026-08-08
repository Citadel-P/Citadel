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
    Task<Result> UpdateNodeAsync(
        UpdateSwarmNodeCommand command,
        CancellationToken cancellationToken = default);

    Task<Result<IReadOnlyList<SwarmServiceResult>>> ListServicesAsync(
        ListSwarmServicesCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmServiceResult>> InspectServiceAsync(
        InspectSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<ManagedSwarmServiceMutationResult>> CreateServiceAsync(
        CreateManagedSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<ManagedSwarmServiceMutationResult>> UpdateServiceAsync(
        UpdateManagedSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> RestartServiceAsync(
        RestartSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> DeleteInventoryServiceAsync(
        DeleteSwarmInventoryServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> DeleteServiceAsync(
        DeleteManagedSwarmServiceCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmLogsResult>> GetServiceLogsAsync(
        GetSwarmServiceLogsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(
        ListSwarmTasksCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmTaskResult>> InspectTaskAsync(
        InspectSwarmTaskCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmLogsResult>> GetTaskLogsAsync(
        GetSwarmTaskLogsCommand command,
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
    Task<Result> CreateSecretAsync(
        CreateSwarmSecretCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> UpdateSecretLabelsAsync(
        UpdateSwarmSecretLabelsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> DeleteSecretAsync(
        DeleteSwarmSecretCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(
        ListSwarmConfigsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<SwarmConfigResult>> InspectConfigAsync(
        InspectSwarmConfigCommand command,
        CancellationToken cancellationToken = default);
    Task<Result<byte[]>> GetConfigDataAsync(
        InspectSwarmConfigCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> CreateConfigAsync(
        CreateSwarmConfigCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> UpdateConfigLabelsAsync(
        UpdateSwarmConfigLabelsCommand command,
        CancellationToken cancellationToken = default);
    Task<Result> DeleteConfigAsync(
        DeleteSwarmConfigCommand command,
        CancellationToken cancellationToken = default);
}
