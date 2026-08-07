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
    public async Task<Result<ManagedSwarmServiceMutationResult>> CreateServiceAsync(CreateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.CreateServiceAsync(SwarmServiceMutationMappers.Map(command), cancellationToken), SwarmServiceMutationMappers.Map);
    public async Task<Result<ManagedSwarmServiceMutationResult>> UpdateServiceAsync(UpdateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.UpdateServiceAsync(SwarmServiceMutationMappers.Map(command), cancellationToken), SwarmServiceMutationMappers.Map);
    public Task<Result> DeleteServiceAsync(DeleteManagedSwarmServiceCommand command, CancellationToken cancellationToken = default)
        => swarmService.DeleteServiceAsync(SwarmServiceMutationMappers.Map(command), cancellationToken);
    public async Task<Result<SwarmLogsResult>> GetServiceLogsAsync(GetSwarmServiceLogsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.GetServiceLogsAsync(command.ServiceId, command.Tail, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(ListSwarmTasksCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListTasksAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmTaskResult>> InspectTaskAsync(InspectSwarmTaskCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectTaskAsync(command.TaskId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmLogsResult>> GetTaskLogsAsync(GetSwarmTaskLogsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.GetTaskLogsAsync(command.TaskId, command.Tail, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmNetworkResult>>> ListNetworksAsync(ListSwarmNetworksCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListNetworksAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmNetworkResult>> InspectNetworkAsync(InspectSwarmNetworkCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectNetworkAsync(command.NetworkId, cancellationToken), SwarmMappers.Map);
    public async Task<Result<IReadOnlyList<SwarmSecretResult>>> ListSecretsAsync(ListSwarmSecretsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListSecretsAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmSecretResult>> InspectSecretAsync(InspectSwarmSecretCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectSecretAsync(command.SecretId, cancellationToken), SwarmMappers.Map);
    public Task<Result> CreateSecretAsync(CreateSwarmSecretCommand command, CancellationToken cancellationToken = default)
        => swarmService.CreateSecretAsync(
            new Hosting.DockerClient.Models.Swarm.CreateSwarmSecretCommand(command.Name, command.Data, command.Labels),
            cancellationToken);
    public Task<Result> UpdateSecretLabelsAsync(UpdateSwarmSecretLabelsCommand command, CancellationToken cancellationToken = default)
        => swarmService.UpdateSecretLabelsAsync(
            new Hosting.DockerClient.Models.Swarm.UpdateSwarmSecretLabelsCommand(command.SecretId, command.VersionIndex, command.Labels),
            cancellationToken);
    public Task<Result> DeleteSecretAsync(DeleteSwarmSecretCommand command, CancellationToken cancellationToken = default)
        => swarmService.DeleteSecretAsync(command.SecretId, cancellationToken);
    public async Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(ListSwarmConfigsCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.ListConfigsAsync(command.Limit, cancellationToken), SwarmMappers.Map);
    public async Task<Result<SwarmConfigResult>> InspectConfigAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default)
        => ServiceResultHandlers.HandleResult(await swarmService.InspectConfigAsync(command.ConfigId, cancellationToken), SwarmMappers.Map);
    public Task<Result<byte[]>> GetConfigDataAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default)
        => swarmService.GetConfigDataAsync(command.ConfigId, cancellationToken);
    public Task<Result> CreateConfigAsync(CreateSwarmConfigCommand command, CancellationToken cancellationToken = default)
        => swarmService.CreateConfigAsync(
            new Hosting.DockerClient.Models.Swarm.CreateSwarmConfigCommand(command.Name, command.Data, command.Labels),
            cancellationToken);
    public Task<Result> UpdateConfigLabelsAsync(UpdateSwarmConfigLabelsCommand command, CancellationToken cancellationToken = default)
        => swarmService.UpdateConfigLabelsAsync(
            new Hosting.DockerClient.Models.Swarm.UpdateSwarmConfigLabelsCommand(command.ConfigId, command.VersionIndex, command.Labels),
            cancellationToken);
    public Task<Result> DeleteConfigAsync(DeleteSwarmConfigCommand command, CancellationToken cancellationToken = default)
        => swarmService.DeleteConfigAsync(command.ConfigId, cancellationToken);
}
