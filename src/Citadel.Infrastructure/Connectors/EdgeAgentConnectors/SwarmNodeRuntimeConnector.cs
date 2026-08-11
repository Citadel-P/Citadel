using System.Runtime.CompilerServices;
using System.Text;
using Citadel.Containers.V1;
using Citadel.Platforms.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using Google.Protobuf;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class SwarmNodeRuntimeConnector(
    IConnectorFactory<IContainerConnector> connectorFactory,
    IEdgeAgentCommandRouter commandRouter) : ISwarmNodeRuntimeConnector
{
    private const int MaximumLogBytes = 4 * 1024 * 1024;
    public async Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return await connectorFactory.GetConnector(platform.ConnectorType).ListContainersAsync(
                new ContainerFilterCommand(platform.Address, All: true),
                cancellationToken);
        }

        var response = await commandRouter.SendUnaryAsync(
            platform.Id,
            dockerNodeId,
            EdgeAgentCommandKind.ContainerList,
            new ListContainersRequest { All = true }.ToByteArray(),
            TimeSpan.FromSeconds(15),
            correlationId: null,
            cancellationToken);
        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<IReadOnlyDictionary<string, DockerContainer>>(
                new ServiceUnavailableError(response.ErrorMessage ?? "The owning Node Agent is unavailable."));
        }

        return ListContainersResponse.Parser.ParseFrom(response.Payload).Map();
    }

    public async Task<Result<ContainerInspectionInfo>> InspectContainerAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(
                new InspectContainerCommand(platform.Address, dockerContainerId),
                cancellationToken);
        }

        var response = await commandRouter.SendUnaryAsync(
            platform.Id,
            dockerNodeId,
            EdgeAgentCommandKind.ContainerInspect,
            new InspectContainerRequest { ContainerId = dockerContainerId }.ToByteArray(),
            TimeSpan.FromSeconds(15),
            correlationId: null,
            cancellationToken);
        if (!response.IsSuccess || response.Payload is null)
        {
            return Result.Failure<ContainerInspectionInfo>(
                new ServiceUnavailableError(response.ErrorMessage ?? "The owning Node Agent is unavailable."));
        }

        return Citadel.SharedModels.V1.InspectContainerResponse.Parser.ParseFrom(response.Payload).Map();
    }

    public async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamContainerLogsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            await foreach (var item in connectorFactory.GetConnector(platform.ConnectorType).StreamLogsAsync(
                               new StreamContainerLogsCommand(platform.Address, dockerContainerId),
                               cancellationToken))
            {
                yield return item;
            }
            yield break;
        }

        var request = new ContainerLogRequest { ContainerId = dockerContainerId };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platform.Id,
                           dockerNodeId,
                           EdgeAgentCommandKind.ContainerLogsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(item.ErrorMessage))
            {
                yield return Encoding.UTF8.GetBytes(item.ErrorMessage);
                yield break;
            }
            if (item.Completed)
                yield break;
            if (item.Payload is { Length: > 0 })
                yield return ContainerLogResponse.Parser.ParseFrom(item.Payload).Log.Memory;
        }
    }

    public async Task<Result<SwarmLogsResult>> GetContainerLogsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        int tail,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return Result.Failure<SwarmLogsResult>(
                new BadRequestError("Manager task logs are read through the Swarm control plane."));
        }

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(TimeSpan.FromSeconds(15));
        var request = new ContainerLogRequest
        {
            ContainerId = dockerContainerId,
            Follow = false,
            Tail = tail
        };
        var lines = new List<string>(Math.Min(tail, 1_000));
        var totalBytes = 0;
        var truncated = false;
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platform.Id,
                           dockerNodeId,
                           EdgeAgentCommandKind.ContainerLogsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           timeout.Token))
        {
            if (!string.IsNullOrWhiteSpace(item.ErrorMessage))
            {
                return Result.Failure<SwarmLogsResult>(new ServiceUnavailableError(item.ErrorMessage));
            }
            if (item.Completed)
                break;
            if (item.Payload is not { Length: > 0 })
                continue;

            var response = ContainerLogResponse.Parser.ParseFrom(item.Payload);
            totalBytes += response.Log.Length;
            if (totalBytes > MaximumLogBytes)
            {
                truncated = true;
                break;
            }
            lines.Add(Encoding.UTF8.GetString(response.Log.Span));
        }

        return Result.Success(new SwarmLogsResult(lines, truncated));
    }

    public async IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        int fetchIntervalMs,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            await foreach (var item in connectorFactory.GetConnector(platform.ConnectorType).StreamContainerStatsAsync(
                               new StreamContainerStatsCommand(dockerContainerId, platform.Address, fetchIntervalMs),
                               cancellationToken))
            {
                yield return item;
            }
            yield break;
        }

        var request = new StreamContainerStatsRequest
        {
            ContainerId = dockerContainerId,
            FetchIntervalMs = fetchIntervalMs
        };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platform.Id,
                           dockerNodeId,
                           EdgeAgentCommandKind.ContainerStatsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
                yield break;
            if (item.Payload is { Length: > 0 })
                yield return Citadel.SharedModels.V1.ContainerMessage.Parser.ParseFrom(item.Payload).Map();
        }
    }

    public async IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(
        Platform platform,
        string dockerNodeId,
        int fetchIntervalMs,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            await foreach (var item in connectorFactory.GetConnector(platform.ConnectorType).StreamContainersStatsAsync(
                               new StreamContainersStatsCommand(platform.Address, fetchIntervalMs),
                               cancellationToken))
            {
                yield return item;
            }
            yield break;
        }

        var request = new StreamContainersStatsRequest { FetchIntervalMs = fetchIntervalMs };
        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platform.Id,
                           dockerNodeId,
                           EdgeAgentCommandKind.ContainersStatsStream,
                           request.ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
                yield break;
            if (item.Payload is { Length: > 0 })
                yield return ContainersStatsResponse.Parser.ParseFrom(item.Payload).Containers.Map();
        }
    }

    public async IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventsAsync(
        Platform platform,
        string dockerNodeId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            yield break;
        }

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platform.Id,
                           dockerNodeId,
                           EdgeAgentCommandKind.PlatformDaemonEventsStream,
                           new Google.Protobuf.WellKnownTypes.Empty().ToByteArray(),
                           Timeout.InfiniteTimeSpan,
                           correlationId: null,
                           cancellationToken))
        {
            if (item.Completed || item.ErrorMessage is not null)
                yield break;
            if (item.Payload is { Length: > 0 })
                yield return DaemonEventResponse.Parser.ParseFrom(item.Payload).Map();
        }
    }

    public async Task<Result> PatchContainersAsync(
        Platform platform,
        string dockerNodeId,
        ContainerAction action,
        IReadOnlyCollection<string> dockerContainerIds,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return await connectorFactory.GetConnector(platform.ConnectorType).PatchAsync(
                new PatchContainerCommand(action, platform.Address, dockerContainerIds),
                cancellationToken);
        }

        var kind = action switch
        {
            ContainerAction.START => EdgeAgentCommandKind.ContainerStart,
            ContainerAction.STOP => EdgeAgentCommandKind.ContainerStop,
            ContainerAction.PAUSE => EdgeAgentCommandKind.ContainerPause,
            ContainerAction.UNPAUSE => EdgeAgentCommandKind.ContainerUnpause,
            ContainerAction.RESTART => EdgeAgentCommandKind.ContainerRestart,
            _ => EdgeAgentCommandKind.Unspecified
        };
        if (kind == EdgeAgentCommandKind.Unspecified)
            return Result.Failure(new BadRequestError("The container action is not supported."));

        var response = await commandRouter.SendUnaryAsync(
            platform.Id,
            dockerNodeId,
            kind,
            new ContainerIds { Ids = { dockerContainerIds } }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);
        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(new ServiceUnavailableError(response.ErrorMessage ?? "The owning Node Agent is unavailable."));
    }

    public async Task<Result> DeleteContainersAsync(
        Platform platform,
        string dockerNodeId,
        IReadOnlyCollection<string> dockerContainerIds,
        bool volumes,
        bool force,
        bool link,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return await connectorFactory.GetConnector(platform.ConnectorType).DeleteAsync(
                new DeleteContainerCommand(dockerContainerIds, platform.Address, volumes, force, link),
                cancellationToken);
        }

        var request = new DeleteContainerRequest
        {
            Ids = { dockerContainerIds },
            V = volumes,
            Force = force,
            Link = link
        };
        var response = await commandRouter.SendUnaryAsync(
            platform.Id,
            dockerNodeId,
            EdgeAgentCommandKind.ContainerDelete,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);
        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(new ServiceUnavailableError(response.ErrorMessage ?? "The owning Node Agent is unavailable."));
    }

    public async Task<IExecSession> ExecAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        string command,
        CancellationToken cancellationToken)
    {
        if (UsesManager(platform, dockerNodeId))
        {
            return await connectorFactory.GetConnector(platform.ConnectorType).ExecAsync(
                platform.Address,
                dockerContainerId,
                command,
                cancellationToken);
        }

        var open = new ExecClientMessage
        {
            Open = new ExecOpen
            {
                ContainerId = dockerContainerId,
                Cmd = { command },
                Tty = true
            }
        };
        var result = await commandRouter.StartInteractiveAsync(
            platform.Id,
            dockerNodeId,
            EdgeAgentCommandKind.ContainerExec,
            open.ToByteArray(),
            Timeout.InfiniteTimeSpan,
            correlationId: null,
            cancellationToken);
        if (!result.IsSuccess(out var interactive, out var error))
            throw new InvalidOperationException(error?.Message ?? "The owning Node Agent is unavailable.");

        return new EdgeExecSession(platform.Id, dockerNodeId, interactive.CommandId, interactive.Output, commandRouter);
    }

    private static bool UsesManager(Platform platform, string dockerNodeId)
        => platform.PlatformDescriptor is DockerSwarmPlatformDescriptor descriptor
           && string.Equals(descriptor.NodeID, dockerNodeId, StringComparison.Ordinal);
}
