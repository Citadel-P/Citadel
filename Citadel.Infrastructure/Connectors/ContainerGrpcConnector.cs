using System.Runtime.CompilerServices;
using Citadel.Agent.Containers.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappings;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using LightResults;
using Microsoft.Extensions.Logging;
using static Citadel.Agent.Containers.V1.ContainerService;

namespace Infrastructure.Connectors;

internal class ContainerGrpcConnector(IGrpcClientFactory grpcClientFactory, ILogger<ContainerGrpcConnector> logger) : IContainerConnector
{
    public async Task<Result<IReadOnlyDictionary<string, Container>>> ListContainersAsync(ContainerFilterCommand command, CancellationToken cancellationToken)
    {
        try
        {
            var containerClient = grpcClientFactory.GetContainerClient(command.PlatformAddress);
            var request = new ListContainersRequest
            {
                All = command?.All,
                Limit = command?.Limit,
                Size = command?.Size,
                Filters = { command?.Filters?.Map() ?? [] }
            };
            
            var containers = await containerClient.ListAsync(request, cancellationToken: cancellationToken);
            return containers.Map(command?.PlatformId ?? throw new ArgumentNullException($"{nameof(command.PlatformId)} should not be null"));
        }
        catch (RpcException ex)
        {
            return Result.Failure<IReadOnlyDictionary<string, Container>>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
    }

    public async Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken)
    {
        try
        {
            var containerClient = grpcClientFactory.GetContainerClient(inspectContainerCommand.PlatformAddress);
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
        var exceptions = new Exception[patchContainerCommand.PlatformContainers.Sum(s => s.Value.Count())];
        var exceptionIndex = 0;
        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = cancellationToken
        };
        await Parallel.ForEachAsync(patchContainerCommand.PlatformContainers, parallelOptions, async(platform, token) =>
        {
            var client = grpcClientFactory.GetContainerClient(platform.Key);
            try
            {
                await ToOperation(client, platform.Value, patchContainerCommand.Action, token);
            }
            catch (Exception ex)
            {
                var idx = Interlocked.Increment(ref exceptionIndex) - 1;
                if (idx < exceptions.Length)
                    exceptions[idx] = ex;
                logger.LogError(ex, "Error while processing container {ContainerIds} on platform {PlatformAddress}", platform.Value, platform.Key);
            }
        });

        static async Task<Google.Protobuf.WellKnownTypes.Empty> ToOperation(ContainerServiceClient client, IEnumerable<string> containerIds, ContainerAction action, CancellationToken cancellationToken) => action switch
        {
            ContainerAction.START => await client.StartAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.STOP => await client.StopAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.PAUSE => await client.PauseAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.UNPAUSE => await client.UnpauseAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            ContainerAction.RESTART => await client.RestartAsync(new ContainerIds() { Ids = { containerIds } }, cancellationToken: cancellationToken),
            _ => throw new NotImplementedException()
        };

        exceptions = [.. exceptions.Where(e => e is not null)]; // Filter out null exceptions

        if (exceptions.Length == 0)
        {
            return Result.Success();
        }
        else
        {
            var rpcException = exceptions.OfType<RpcException>().FirstOrDefault();
            return rpcException is not null
                ? Result.Failure(new ClientRpcException($"An RPC exception occurred: {rpcException.Message}", rpcException.StatusCode))
                : Result.Failure(new InternalServerError($"An error occurred while processing the request,  {exceptions.First().Message}"));
        }
    }

    public async Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
    {
        var exceptions = new Exception[deleteContainerCommand.PlatformContainers.Sum(s => s.Value.Count())];
        var exceptionIndex = 0;
        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = cancellationToken
        };
        await Parallel.ForEachAsync(deleteContainerCommand.PlatformContainers, parallelOptions, async (platform, ct) =>
        {
            var client = grpcClientFactory.GetContainerClient(platform.Key);
            try
            {
                var rpcRequest = new DeleteContainerRequest
                {
                    Ids = { platform.Value },
                    V = deleteContainerCommand.Verbose ?? false,
                    Force = deleteContainerCommand.Force ?? false,
                    Link = deleteContainerCommand.Link ?? false,
                };
                await client.DeleteAsync(rpcRequest, cancellationToken: ct);
            }
            catch (Exception ex)
            {
                var idx = Interlocked.Increment(ref exceptionIndex) - 1;
                if (idx < exceptions.Length)
                    exceptions[idx] = ex;
                logger.LogError(ex, "Error while processing container {ContainerIds} on platform {PlatformAddress}", platform.Value, platform.Key);
            }
        });

        exceptions = [.. exceptions.Where(e => e is not null)]; // Filter out null exceptions

        if (exceptions.Length == 0)
        {
            return Result.Success();
        }
        else
        {
            var rpcException = exceptions.OfType<RpcException>().FirstOrDefault();
            return rpcException is not null
                ? Result.Failure(new ClientRpcException($"An RPC exception occurred: {rpcException.Message}", rpcException.StatusCode))
                : Result.Failure(new InternalServerError($"An error occurred while processing the request, {exceptions.First().Message}"));
        }
    }

    public async IAsyncEnumerable<ContainerLogInfo> StreamLogsAsync(StreamContainerLogsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = grpcClientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerLogs(new ContainerLogRequest() { ContainerId = command .ContainerId}, cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map();
        }
    }

    public async IAsyncEnumerable<ContainerStats> StreamContainerStatsAsync(StreamContainerStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = grpcClientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerStats(new ContainerStatsRequest() { FetchIntervalMs = command .FetchIntervalMs}, cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map();
        }
    }

    public async IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(StreamDaemonEventCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = grpcClientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamDaemonEvent(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Map(command.PlatformId);
        }
    }
}
