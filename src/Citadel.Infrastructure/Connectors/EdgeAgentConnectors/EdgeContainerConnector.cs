using System.Runtime.CompilerServices;
using System.Text;
using Citadel.Containers.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Google.Protobuf;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeContainerConnector(IEdgeAgentCommandRouter commandRouter) : IContainerConnector
{
    public async Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(ContainerFilterCommand containerFilterCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(containerFilterCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<IReadOnlyDictionary<string, DockerContainer>>(addressError!);
        }

        var request = new ListContainersRequest
        {
            All = containerFilterCommand.All,
            Limit = containerFilterCommand.Limit,
            Size = containerFilterCommand.Size,
            Filters = { containerFilterCommand.Filters?.Map() ?? [] }
        };

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerList,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<IReadOnlyDictionary<string, DockerContainer>>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ContainerList, response));
        }

        return ListContainersResponse.Parser.ParseFrom(response.Payload).Map();
    }

    public async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(streamContainerLogsCommand.PlatformAddress, out var platformId, out _))
        {
            yield return Encoding.UTF8.GetBytes("Edge Agent platform address is invalid.");
            yield break;
        }

        var request = new ContainerLogRequest { ContainerId = streamContainerLogsCommand.ContainerId };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.ContainerLogsStream,
                           request.ToByteArray(),
                           TimeSpan.FromMinutes(10),
                           correlationId: null,
                           cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(item.ErrorMessage))
            {
                yield return Encoding.UTF8.GetBytes(item.ErrorMessage);
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                var response = ContainerLogResponse.Parser.ParseFrom(item.Payload);
                yield return response.Log.Memory;
            }
        }
    }

    public async Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(inspectContainerCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<ContainerInspectionInfo>(addressError!);
        }

        var request = new InspectContainerRequest { ContainerId = inspectContainerCommand.ContainerId };
        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerInspect,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<ContainerInspectionInfo>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ContainerInspect, response));
        }

        return Citadel.SharedModels.V1.InspectContainerResponse.Parser.ParseFrom(response.Payload).Map();
    }

    public async Task<Result<string>> CreateAsync(CreateContainerCommand createContainerCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(createContainerCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure<string>(addressError!);
        }

        Dictionary<string, Citadel.SharedModels.V1.EndpointSettings>? networks = [];
        foreach (var (key, value) in createContainerCommand.Networks ?? [])
        {
            networks[key] = value.MapAgent();
        }

        var request = new CreateContainerRequest
        {
            ImageId = createContainerCommand.ImageId,
            Name = createContainerCommand.Name ?? string.Empty,
            WorkingDir = createContainerCommand.WorkingDir,
            User = createContainerCommand.User,
            MemoryLimit = createContainerCommand.MemoryLimit,
            CpuQuota = createContainerCommand.CpuQuota,
            MemoryReservation = createContainerCommand.MemoryReservation,
            MemorySwap = createContainerCommand.MemorySwap,
            PidsLimit = createContainerCommand.PidsLimit,
            AutoRemove = createContainerCommand.AutoRemove ?? false,
            Privileged = createContainerCommand.Privileged ?? false,
            ReadonlyRootfs = createContainerCommand.ReadonlyRootfs,
            RestartPolicy = createContainerCommand.RestartPolicy.Map(),
            Labels = { createContainerCommand.Labels ?? [] },
            EnvVars = { createContainerCommand.EnvVars ?? [] },
            Ports = { createContainerCommand.Ports ?? [] },
            Volumes = { createContainerCommand.Volumes ?? [] },
            Mounts = { createContainerCommand.Mounts?.Select(mount => mount.MapAgent()) ?? [] },
            CapAdd = { createContainerCommand.CapAdd ?? [] },
            CapDrop = { createContainerCommand.CapDrop ?? [] },
            SecurityOpt = { createContainerCommand.SecurityOpt ?? [] },
            NetworkMode = createContainerCommand.NetworkMode,
            Networks = { networks },
            EntryPoint = { createContainerCommand.EntryPoint ?? [] },
            Command = { createContainerCommand.Command ?? [] }
        };

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerCreate,
            request.ToByteArray(),
            TimeSpan.FromMinutes(2),
            correlationId: null,
            cancellationToken);

        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<string>(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ContainerCreate, response));
        }

        return CreateContainerResponse.Parser.ParseFrom(response.Payload).ContainerId;
    }

    public async Task<Result> PatchAsync(PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(patchContainerCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure(addressError!);
        }

        var kind = MapEdgeCommandKind(patchContainerCommand.Action);
        var response = await commandRouter.SendUnaryAsync(
            platformId,
            kind,
            new ContainerIds { Ids = { patchContainerCommand.ContainerIds } }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(EdgeConnectorHelpers.CommandFailure(kind, response));
    }

    public async Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(deleteContainerCommand.PlatformAddress, out var platformId, out var addressError))
        {
            return Result.Failure(addressError!);
        }

        var request = new DeleteContainerRequest
        {
            Ids = { deleteContainerCommand.ContainerIds },
            V = deleteContainerCommand.Volume ?? false,
            Force = deleteContainerCommand.Force ?? false,
            Link = deleteContainerCommand.Link ?? false
        };
        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.ContainerDelete,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.ContainerDelete, response));
    }

    public async Task<IExecSession> ExecAsync(string platformAddress, string containerId, string cmd, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out var addressError))
        {
            throw new InvalidOperationException(addressError?.Message ?? "Edge Agent platform address is invalid.");
        }

        var open = new ExecClientMessage
        {
            Open = new ExecOpen
            {
                ContainerId = containerId,
                Cmd = { cmd },
                Tty = true
            }
        };

        var result = await commandRouter.StartInteractiveAsync(
            platformId,
            EdgeAgentCommandKind.ContainerExec,
            open.ToByteArray(),
            Timeout.InfiniteTimeSpan,
            correlationId: null,
            cancellationToken);

        if (!result.IsSuccess(out var command, out var error))
        {
            throw new InvalidOperationException(error?.Message ?? "Failed to start Edge Agent exec session.");
        }

        return new EdgeExecSession(platformId, command.CommandId, command.Output, commandRouter);
    }

    public Task<Result<ContainerBinaryExecResult>> ExecBinaryAsync(string platformAddress, ContainerBinaryExecRequest request, CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out var addressError))
        {
            return Task.FromResult(Result.Failure<ContainerBinaryExecResult>(addressError!));
        }

        var rpcRequest = new ExecBinaryRequest
        {
            ContainerId = request.ContainerId,
            Cmd = { request.Command },
            Env = { request.Environment?.ToDictionary(kvp => kvp.Key, kvp => kvp.Value) ?? [] },
            AttachStdout = request.AttachStdout,
            AttachStderr = request.AttachStderr,
            Tty = request.Tty
        };

        var stream = commandRouter.SendServerStreamAsync(
            platformId,
            EdgeAgentCommandKind.ContainerExecBinary,
            rpcRequest.ToByteArray(),
            Timeout.InfiniteTimeSpan,
            correlationId: null,
            cancellationToken);

        var exitCode = new TaskCompletionSource<int?>(TaskCreationOptions.RunContinuationsAsynchronously);

        return Task.FromResult(Result.Success<ContainerBinaryExecResult>(new ContainerBinaryExecResult
        {
            Output = ReadBinaryOutputAsync(stream, exitCode, cancellationToken),
            GetExitCodeAsync = async ct => await exitCode.Task.WaitAsync(ct).ConfigureAwait(false),
            CleanupAsync = () =>
            {
                exitCode.TrySetCanceled();
                return ValueTask.CompletedTask;
            }
        }));
    }

    public async IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(
        StreamContainerStatsCommand streamStatsCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(streamStatsCommand.PlatformAddress, out var platformId, out _))
        {
            yield break;
        }

        var request = new StreamContainerStatsRequest
        {
            ContainerId = streamStatsCommand.ContainerId,
            FetchIntervalMs = streamStatsCommand.FetchIntervalMs
        };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.ContainerStatsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return Citadel.SharedModels.V1.ContainerMessage.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }

    public async IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(
        StreamContainersStatsCommand streamStatsCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(streamStatsCommand.PlatformAddress, out var platformId, out _))
        {
            yield break;
        }

        var request = new StreamContainersStatsRequest { FetchIntervalMs = streamStatsCommand.FetchIntervalMs };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.ContainersStatsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return ContainersStatsResponse.Parser.ParseFrom(item.Payload).Containers.Map();
            }
        }
    }

    private static EdgeAgentCommandKind MapEdgeCommandKind(ContainerAction action)
        => action switch
        {
            ContainerAction.START => EdgeAgentCommandKind.ContainerStart,
            ContainerAction.STOP => EdgeAgentCommandKind.ContainerStop,
            ContainerAction.PAUSE => EdgeAgentCommandKind.ContainerPause,
            ContainerAction.UNPAUSE => EdgeAgentCommandKind.ContainerUnpause,
            ContainerAction.RESTART => EdgeAgentCommandKind.ContainerRestart,
            _ => EdgeAgentCommandKind.Unspecified
        };

    private static async IAsyncEnumerable<ContainerBinaryExecChunk> ReadBinaryOutputAsync(
        IAsyncEnumerable<EdgeAgentStreamItem> stream,
        TaskCompletionSource<int?> exitCode,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var item in stream.WithCancellation(cancellationToken))
            {
                if (!string.IsNullOrWhiteSpace(item.ErrorMessage))
                {
                    exitCode.TrySetResult(1);
                    yield return new ContainerBinaryExecChunk(
                        ContainerExecStream.Stderr,
                        Encoding.UTF8.GetBytes(item.ErrorMessage));
                    yield break;
                }

                if (item.Completed)
                {
                    exitCode.TrySetResult(null);
                    yield break;
                }

                if (item.Payload is not { Length: > 0 })
                {
                    continue;
                }

                var message = ExecServerMessage.Parser.ParseFrom(item.Payload);
                switch (message.MsgCase)
                {
                    case ExecServerMessage.MsgOneofCase.Output when message.Output?.Data.Length > 0:
                        yield return new ContainerBinaryExecChunk(
                            message.Output.Stream == StreamType.Stderr ? ContainerExecStream.Stderr : ContainerExecStream.Stdout,
                            message.Output.Data.Memory);
                        break;

                    case ExecServerMessage.MsgOneofCase.Error:
                        exitCode.TrySetResult(1);
                        yield return new ContainerBinaryExecChunk(
                            ContainerExecStream.Stderr,
                            Encoding.UTF8.GetBytes(message.Error?.Message ?? "Binary exec failed."));
                        yield break;

                    case ExecServerMessage.MsgOneofCase.Exit:
                        exitCode.TrySetResult(message.Exit?.ExitCode);
                        break;
                }
            }

            exitCode.TrySetResult(null);
        }
        finally
        {
            exitCode.TrySetResult(null);
        }
    }
}

internal sealed class EdgeExecSession(
    Guid platformId,
    string commandId,
    IAsyncEnumerable<EdgeAgentStreamItem> output,
    IEdgeAgentCommandRouter commandRouter)
    : IExecSession
{
    public IAsyncEnumerable<ReadOnlyMemory<byte>> Output => ReadOutputAsync();

    private async IAsyncEnumerable<ReadOnlyMemory<byte>> ReadOutputAsync([EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await foreach (var item in output.WithCancellation(cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(item.ErrorMessage))
            {
                yield return Encoding.UTF8.GetBytes(item.ErrorMessage);
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is not { Length: > 0 })
            {
                continue;
            }

            var message = ExecServerMessage.Parser.ParseFrom(item.Payload);
            switch (message.MsgCase)
            {
                case ExecServerMessage.MsgOneofCase.Output when message.Output?.Data.Length > 0:
                    yield return message.Output.Data.Memory;
                    break;
                case ExecServerMessage.MsgOneofCase.Error:
                    yield return Encoding.UTF8.GetBytes(message.Error?.Message ?? "Exec error");
                    yield break;
                case ExecServerMessage.MsgOneofCase.Exit:
                    yield return Encoding.UTF8.GetBytes($"[exit {message.Exit?.ExitCode ?? 0}]");
                    yield break;
            }
        }
    }

    public async Task SendAsync(ReadOnlyMemory<byte> input, CancellationToken ct)
    {
        var message = new ExecClientMessage
        {
            Stdin = new ExecStdin { Data = ByteString.CopyFrom(input.Span) }
        };

        var result = await commandRouter.SendStreamInputAsync(platformId, commandId, message.ToByteArray(), ct);
        if (result.IsFailure(out var error))
        {
            throw new InvalidOperationException(error.Message);
        }
    }

    public async Task ResizeAsync(int cols, int rows, CancellationToken ct)
    {
        var message = new ExecClientMessage
        {
            Resize = new ExecResize { Cols = cols, Rows = rows }
        };

        var result = await commandRouter.SendStreamInputAsync(platformId, commandId, message.ToByteArray(), ct);
        if (result.IsFailure(out var error))
        {
            throw new InvalidOperationException(error.Message);
        }
    }

    public async ValueTask DisposeAsync()
        => await commandRouter.CancelAsync(platformId, commandId, "Exec session disposed.", CancellationToken.None);
}
