using Citadel.Containers.V1;
using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Google.Protobuf;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using LightResults;
using System.Runtime.CompilerServices;
using System.Text;
using static Citadel.Containers.V1.ContainerService;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentContainerConnector(IGrpcClientFactory clientFactory) : IContainerConnector
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

    public async Task<Result<string>> CreateAsync(CreateContainerCommand createContainerCommand, CancellationToken cancellationToken)
    {
        try
        {
            var containerClient = clientFactory.GetContainerClient(createContainerCommand.PlatformAddress);
            Dictionary<string, Citadel.SharedModels.V1.EndpointSettings>? networks = [];
            foreach (var (k, v) in createContainerCommand.Networks ?? [])
            {
                networks[k] = v.MapAgent();
            }
            var request = new CreateContainerRequest()
            {
                ImageId = createContainerCommand.ImageId,
                Name = createContainerCommand.Name,
                WorkingDir = createContainerCommand.WorkingDir,
                User = createContainerCommand.User,
                MemoryLimit = createContainerCommand.MemoryLimit,
                CpuQuota = createContainerCommand.CpuQuota,
                MemoryReservation = createContainerCommand.MemoryReservation,
                AutoRemove = createContainerCommand.AutoRemove ?? false,
                RestartPolicy = createContainerCommand.RestartPolicy.Map(),
                Labels = { createContainerCommand.Labels ?? [] },
                EnvVars = { createContainerCommand.EnvVars ?? [] },
                Ports = { createContainerCommand.Ports ?? [] },
                Volumes = { createContainerCommand.Volumes ?? [] },
                Networks = { networks },
                EntryPoint = { createContainerCommand.EntryPoint ?? [] },
                Command = { createContainerCommand.Command ?? [] }
            };

            var result = await containerClient.CreateAsync(request, cancellationToken: cancellationToken);
            
            return result.ContainerId;
        }
        catch (RpcException ex)
        {
            return Result.Failure<string>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
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

    public async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerLogs(new ContainerLogRequest() { ContainerId = command.ContainerId }, cancellationToken: cancellationToken);
        await foreach (var response in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return response.Log.Memory;
        }
    }

    public async IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(StreamContainersStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainersStats(new StreamContainersStatsRequest() { FetchIntervalMs = command.FetchIntervalMs }, cancellationToken: cancellationToken);
        await foreach (var result in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return result.Containers.Map();
        }
    }

    public async IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(StreamContainerStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(command.PlatformAddress);
        using var streamCall = containerClient.StreamContainerStats(new StreamContainerStatsRequest() { ContainerId = command.ContainerId, FetchIntervalMs = command.FetchIntervalMs }, cancellationToken: cancellationToken);
        await foreach (var result in streamCall.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return result.Map();
        }
    }

    public async Task<IExecSession> ExecAsync(string platformAddress, string containerId, string cmd, CancellationToken cancellationToken)
    {
        var containerClient = clientFactory.GetContainerClient(platformAddress);
        var call = containerClient.Exec(cancellationToken: cancellationToken);

        var open = new ExecClientMessage
        {
            Open = new ExecOpen
            {
                ContainerId = containerId,
                Cmd = { cmd },
                Tty = true
            }
        };

        await call.RequestStream.WriteAsync(open, cancellationToken).ConfigureAwait(false);

        return new AgentExecSession(call);
    }
}

internal sealed class AgentExecSession(AsyncDuplexStreamingCall<ExecClientMessage, ExecServerMessage> call) : IExecSession
{
    private readonly AsyncDuplexStreamingCall<ExecClientMessage, ExecServerMessage> _call = call;

    public IAsyncEnumerable<ReadOnlyMemory<byte>> Output => ReadOutputAsync();

    private async IAsyncEnumerable<ReadOnlyMemory<byte>> ReadOutputAsync([EnumeratorCancellation] CancellationToken ct = default)
    {
        await foreach (var msg in _call.ResponseStream.ReadAllAsync(ct))
        {
            switch (msg.MsgCase)
            {
                case ExecServerMessage.MsgOneofCase.Output when msg.Output?.Data.Length > 0:
                    yield return msg.Output.Data.Memory;
                    break;

                case ExecServerMessage.MsgOneofCase.Error:
                    yield return Encoding.UTF8.GetBytes(msg.Error?.Message ?? "Exec error");
                    yield break;

                case ExecServerMessage.MsgOneofCase.Exit:
                    yield return Encoding.UTF8.GetBytes($"[exit {msg.Exit?.ExitCode ?? 0}]");
                    yield break;
            }
        }
    }

    public Task SendAsync(ReadOnlyMemory<byte> input, CancellationToken ct) =>
        _call.RequestStream.WriteAsync(new ExecClientMessage
        {
            Stdin = new ExecStdin { Data = ByteString.CopyFrom(input.Span) }
        }, ct);

    public Task ResizeAsync(int cols, int rows, CancellationToken ct) =>
        _call.RequestStream.WriteAsync(new ExecClientMessage
        {
            Resize = new ExecResize { Cols = cols, Rows = rows }
        }, ct);

    public async ValueTask DisposeAsync()
    {
        try
        {
            await _call.RequestStream.CompleteAsync().ConfigureAwait(false);
        }
        catch { }

        _call.Dispose();
    }
}
