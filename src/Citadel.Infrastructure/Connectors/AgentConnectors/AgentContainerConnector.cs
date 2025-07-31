using System.Runtime.CompilerServices;
using Citadel.Agent.Containers.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Hosting.Common.ObjectPoolManager;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Services;
using LightResults;
using static Citadel.Agent.Containers.V1.ContainerService;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentContainerConnector(IGrpcClientFactory clientFactory, IObjectPoolManager objectPoolManager) : IContainerConnector
{
    public async Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(ContainerFilterCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
            var request = new ListContainersRequest
            {
                All = command?.All,
                Limit = command?.Limit,
                Size = command?.Size,
                Filters = { command?.Filters?.Map() ?? [] }
            };
            
            var containers = await containerClient.ListAsync(request, cancellationToken: cancellationToken);
            return containers.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<IReadOnlyDictionary<string, DockerContainer>>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken)
    {
        try
        {
            var containerClient = clientFactory.GetContainerClient(inspectContainerCommand.PlatformAddress);
            var request = new InspectContainerRequest() { ContainerId = inspectContainerCommand.ContainerId };
            var result = await containerClient.InspectAsync(request, cancellationToken: cancellationToken);
            return result.Map();
        }
        catch (RpcException ex)
        {
            return Result.Failure<ContainerInspectionInfo>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result> PatchAsync(PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken)
    {
        try
        {
            var client = clientFactory.GetContainerClient(patchContainerCommand.PlatformAddress);
            await ToOperation(client, patchContainerCommand.ContainerIds, patchContainerCommand.Action, cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }

        static async Task<Google.Protobuf.WellKnownTypes.Empty> ToOperation(ContainerServiceClient client, IEnumerable<string> containerIds, ContainerAction action, CancellationToken cancellationToken) => action switch
        {
            ContainerAction.START => await client.StartAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.STOP => await client.StopAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.PAUSE => await client.PauseAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.UNPAUSE => await client.UnpauseAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.RESTART => await client.RestartAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            _ => throw new NotImplementedException()
        };

        return Result.Success();
    }

    public async Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
    {
        var client = clientFactory.GetContainerClient(deleteContainerCommand.PlatformAddress);
        try
        {
            var rpcRequest = new DeleteContainerRequest
            {
                Ids = { deleteContainerCommand.ContainerIds },
                V = deleteContainerCommand.Volume ?? false,
                Force = deleteContainerCommand.Force ?? false,
                Link = deleteContainerCommand.Link ?? false,
            };
            await client.DeleteAsync(rpcRequest, cancellationToken: cancellationToken);
            return Result.Success();
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }

    public async IAsyncEnumerable<ContainerLogInfo> StreamLogsAsync(StreamContainerLogsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerLogs(new ContainerLogRequest() { ContainerId = command .ContainerId}, cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map();
        }
    }

    public async IAsyncEnumerable<PooledHandle<Dictionary<string, DockerContainerStat>>> StreamContainersStatsAsync(StreamContainersStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerStats(new ContainerStatsRequest() { FetchIntervalMs = command.FetchIntervalMs }, cancellationToken: cancellationToken);
        await foreach (var result in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            var pooledDictionary = objectPoolManager.GetPooled<Dictionary<string, DockerContainerStat>>();
            var dictionary = pooledDictionary.Value;
            dictionary.Clear();

            foreach (var kvp in result.Containers)
            {
                var stat = objectPoolManager.Get<DockerContainerStat>();
                kvp.Value.Map(stat);
                dictionary.Add(kvp.Key, stat);
            }

            yield return pooledDictionary;
        }
    }

    public IAsyncEnumerable<PooledHandle<DockerContainer>> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
