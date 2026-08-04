using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal sealed class LocalSwarmConnector(ISwarmService swarmService) : ISwarmConnector
{
    public async Task<Result<IReadOnlyList<SwarmNodeResult>>> ListNodesAsync(
        ListSwarmNodesCommand command,
        CancellationToken cancellationToken = default)
    {
        var result = await swarmService.ListNodesAsync(
            command.Limit,
            command.IncludeTaskCounts,
            cancellationToken);
        return ServiceResultHandlers.HandleResult(result, SwarmMappers.Map);
    }

    public async Task<Result<SwarmNodeResult>> InspectNodeAsync(
        InspectSwarmNodeCommand command,
        CancellationToken cancellationToken = default)
    {
        var result = await swarmService.InspectNodeAsync(command.NodeId, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, SwarmMappers.Map);
    }

    public async Task<Result<IReadOnlyList<SwarmServiceResult>>> ListServicesAsync(ListSwarmServicesCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListServicesAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmServiceResult>> InspectServiceAsync(InspectSwarmServiceCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectServiceAsync(command.ServiceId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(ListSwarmTasksCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListTasksAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmTaskResult>> InspectTaskAsync(InspectSwarmTaskCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectTaskAsync(command.TaskId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmNetworkResult>>> ListNetworksAsync(ListSwarmNetworksCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListNetworksAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmNetworkResult>> InspectNetworkAsync(InspectSwarmNetworkCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectNetworkAsync(command.NetworkId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmSecretResult>>> ListSecretsAsync(ListSwarmSecretsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListSecretsAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmSecretResult>> InspectSecretAsync(InspectSwarmSecretCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectSecretAsync(command.SecretId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(ListSwarmConfigsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListConfigsAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmConfigResult>> InspectConfigAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectConfigAsync(command.ConfigId, cancellationToken), SwarmMappers.Map);
}
