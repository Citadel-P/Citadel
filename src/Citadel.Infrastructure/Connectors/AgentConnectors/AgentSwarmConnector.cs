using Citadel.Swarm.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Grpc.Core;
using Google.Protobuf;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;

namespace Infrastructure.Connectors.AgentConnectors;

internal sealed class AgentSwarmConnector(IGrpcClientFactory clientFactory) : ISwarmConnector
{
    public async Task<Result<IReadOnlyList<SwarmNodeResult>>> ListNodesAsync(
        ListSwarmNodesCommand command,
        CancellationToken cancellationToken = default)
    {
        try
        {
            var client = clientFactory.GetSwarmClient(command.PlatformAddress);
            return Result.Success<IReadOnlyList<SwarmNodeResult>>(
                (await client.ListNodesAsync(
                    new ListSwarmNodesRequest
                    {
                        MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit),
                        IncludeTaskCounts = command.IncludeTaskCounts
                    },
                    cancellationToken: cancellationToken)).Map());
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (RpcException exception) when (cancellationToken.IsCancellationRequested)
        {
            throw new OperationCanceledException("The Swarm node request was cancelled.", exception, cancellationToken);
        }
        catch (RpcException exception)
        {
            return Result.Failure<IReadOnlyList<SwarmNodeResult>>(
                new ClientRpcException($"An error occurred while listing Swarm nodes, {exception.Message}", exception.StatusCode));
        }
    }

    public async Task<Result<SwarmNodeResult>> InspectNodeAsync(
        InspectSwarmNodeCommand command,
        CancellationToken cancellationToken = default)
    {
        try
        {
            var client = clientFactory.GetSwarmClient(command.PlatformAddress);
            return (await client.InspectNodeAsync(
                new InspectSwarmNodeRequest { NodeId = command.NodeId },
                cancellationToken: cancellationToken)).Map();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (RpcException exception) when (cancellationToken.IsCancellationRequested)
        {
            throw new OperationCanceledException("The Swarm node request was cancelled.", exception, cancellationToken);
        }
        catch (RpcException exception)
        {
            return Result.Failure<SwarmNodeResult>(
                new ClientRpcException($"An error occurred while inspecting the Swarm node, {exception.Message}", exception.StatusCode));
        }
    }

    public Task<Result<IReadOnlyList<SwarmServiceResult>>> ListServicesAsync(ListSwarmServicesCommand command, CancellationToken cancellationToken = default) =>
        ExecuteListAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).ListServicesAsync(new ListSwarmServicesRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "services", cancellationToken);
    public Task<Result<SwarmServiceResult>> InspectServiceAsync(InspectSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).InspectServiceAsync(new InspectSwarmServiceRequest { ServiceId = command.ServiceId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "service", cancellationToken);
    public Task<Result<ManagedSwarmServiceMutationResult>> CreateServiceAsync(CreateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).CreateServiceAsync(SwarmServiceTransportMappers.Map(command), cancellationToken: cancellationToken).ResponseAsync, SwarmServiceTransportMappers.Map, "service mutation", cancellationToken);
    public Task<Result<ManagedSwarmServiceMutationResult>> UpdateServiceAsync(UpdateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).UpdateServiceAsync(SwarmServiceTransportMappers.Map(command), cancellationToken: cancellationToken).ResponseAsync, SwarmServiceTransportMappers.Map, "service mutation", cancellationToken);
    public Task<Result> DeleteServiceAsync(DeleteManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).DeleteServiceAsync(SwarmServiceTransportMappers.Map(command), cancellationToken: cancellationToken).ResponseAsync, "service", cancellationToken);
    public Task<Result<SwarmLogsResult>> GetServiceLogsAsync(GetSwarmServiceLogsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).GetServiceLogsAsync(new SwarmLogsRequest { ResourceId = command.ServiceId, Tail = SwarmInventoryLimits.NormalizeLogLines(command.Tail) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "service logs", cancellationToken);
    public Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(ListSwarmTasksCommand command, CancellationToken cancellationToken = default) =>
        ExecuteListAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).ListTasksAsync(new ListSwarmTasksRequest { MaxItems = SwarmInventoryLimits.Normalize(command.Limit) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "tasks", cancellationToken);
    public Task<Result<SwarmTaskResult>> InspectTaskAsync(InspectSwarmTaskCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).InspectTaskAsync(new InspectSwarmTaskRequest { TaskId = command.TaskId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "task", cancellationToken);
    public Task<Result<SwarmLogsResult>> GetTaskLogsAsync(GetSwarmTaskLogsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).GetTaskLogsAsync(new SwarmLogsRequest { ResourceId = command.TaskId, Tail = SwarmInventoryLimits.NormalizeLogLines(command.Tail) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "task logs", cancellationToken);
    public Task<Result<IReadOnlyList<SwarmNetworkResult>>> ListNetworksAsync(ListSwarmNetworksCommand command, CancellationToken cancellationToken = default) =>
        ExecuteListAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).ListNetworksAsync(new ListSwarmNetworksRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "networks", cancellationToken);
    public Task<Result<SwarmNetworkResult>> InspectNetworkAsync(InspectSwarmNetworkCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).InspectNetworkAsync(new InspectSwarmNetworkRequest { NetworkId = command.NetworkId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "network", cancellationToken);
    public Task<Result<IReadOnlyList<SwarmSecretResult>>> ListSecretsAsync(ListSwarmSecretsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteListAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).ListSecretsAsync(new ListSwarmSecretsRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "secrets", cancellationToken);
    public Task<Result<SwarmSecretResult>> InspectSecretAsync(InspectSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).InspectSecretAsync(new InspectSwarmSecretRequest { SecretId = command.SecretId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "secret", cancellationToken);
    public Task<Result> CreateSecretAsync(CreateSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).CreateSecretAsync(new CreateSwarmSecretRequest
        {
            Name = command.Name,
            Data = Google.Protobuf.ByteString.CopyFrom(command.Data),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken: cancellationToken).ResponseAsync, "secret", cancellationToken);
    public Task<Result> UpdateSecretLabelsAsync(UpdateSwarmSecretLabelsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).UpdateSecretLabelsAsync(new UpdateSwarmResourceLabelsRequest
        {
            ResourceId = command.SecretId,
            VersionIndex = checked((ulong)command.VersionIndex),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken: cancellationToken).ResponseAsync, "secret", cancellationToken);
    public Task<Result> DeleteSecretAsync(DeleteSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).DeleteSecretAsync(
            new DeleteSwarmSecretRequest { SecretId = command.SecretId }, cancellationToken: cancellationToken).ResponseAsync,
            "secret", cancellationToken);
    public Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(ListSwarmConfigsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteListAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).ListConfigsAsync(new ListSwarmConfigsRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "configs", cancellationToken);
    public Task<Result<SwarmConfigResult>> InspectConfigAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).InspectConfigAsync(new InspectSwarmConfigRequest { ConfigId = command.ConfigId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Map(), "config", cancellationToken);
    public Task<Result<byte[]>> GetConfigDataAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        ExecuteAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).GetConfigDataAsync(new InspectSwarmConfigRequest { ConfigId = command.ConfigId }, cancellationToken: cancellationToken).ResponseAsync, static value => value.Data.ToByteArray(), "config data", cancellationToken);
    public Task<Result> CreateConfigAsync(CreateSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).CreateConfigAsync(new CreateSwarmConfigRequest
        {
            Name = command.Name,
            Data = Google.Protobuf.ByteString.CopyFrom(command.Data),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken: cancellationToken).ResponseAsync, "config", cancellationToken);
    public Task<Result> UpdateConfigLabelsAsync(UpdateSwarmConfigLabelsCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).UpdateConfigLabelsAsync(new UpdateSwarmResourceLabelsRequest
        {
            ResourceId = command.ConfigId,
            VersionIndex = checked((ulong)command.VersionIndex),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken: cancellationToken).ResponseAsync, "config", cancellationToken);
    public Task<Result> DeleteConfigAsync(DeleteSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        ExecuteMutationAsync(() => clientFactory.GetSwarmClient(command.PlatformAddress).DeleteConfigAsync(
            new DeleteSwarmConfigRequest { ConfigId = command.ConfigId }, cancellationToken: cancellationToken).ResponseAsync,
            "config", cancellationToken);

    private static async Task<Result<IReadOnlyList<T>>> ExecuteListAsync<TResponse, T>(Func<Task<TResponse>> call, Func<TResponse, IReadOnlyList<T>> map, string resource, CancellationToken cancellationToken)
    {
        try { return Result.Success(map(await call())); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (RpcException exception) when (cancellationToken.IsCancellationRequested) { throw new OperationCanceledException($"The Swarm {resource} request was cancelled.", exception, cancellationToken); }
        catch (RpcException exception) { return Result.Failure<IReadOnlyList<T>>(new ClientRpcException($"An error occurred while listing Swarm {resource}, {exception.Message}", exception.StatusCode)); }
    }

    private static async Task<Result<T>> ExecuteAsync<TResponse, T>(Func<Task<TResponse>> call, Func<TResponse, T> map, string resource, CancellationToken cancellationToken)
    {
        try { return Result.Success(map(await call())); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (RpcException exception) when (cancellationToken.IsCancellationRequested) { throw new OperationCanceledException($"The Swarm {resource} request was cancelled.", exception, cancellationToken); }
        catch (RpcException exception) { return Result.Failure<T>(new ClientRpcException($"An error occurred while inspecting the Swarm {resource}, {exception.Message}", exception.StatusCode)); }
    }

    private static async Task<Result> ExecuteMutationAsync<TResponse>(
        Func<Task<TResponse>> call,
        string resource,
        CancellationToken cancellationToken)
    {
        try
        {
            await call();
            return Result.Success();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (RpcException exception) when (cancellationToken.IsCancellationRequested)
        {
            throw new OperationCanceledException($"The Swarm {resource} request was cancelled.", exception, cancellationToken);
        }
        catch (RpcException exception)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while mutating the Swarm {resource}, {exception.Message}", exception.StatusCode));
        }
    }
}
