using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using LightResults;
using Microsoft.Extensions.Logging;
using ProtoEdgeCommandKind = Citadel.Edge.V1.EdgeCommandKind;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentCommandRouter(
    EdgeAgentSessionRegistry registry,
    ILogger<EdgeAgentCommandRouter> logger) : IEdgeAgentCommandRouter
{
    public async Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            return EdgeAgentCommandRouterResult.Failure("Edge Agent command payload exceeded the maximum payload size.");
        }

        if (!registry.TryGet(platformId, out var session))
        {
            return EdgeAgentCommandRouterResult.Failure("Edge Agent is offline.");
        }

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(timeout);

        EdgePendingCommand pending;
        try
        {
            pending = await session.SendCommandAsync(
                MapKind(kind),
                payload,
                timeout,
                correlationId,
                expectsStream: false,
                timeoutCts.Token);
            logger.LogDebug("Sent Edge unary command {CommandId} {CommandKind} to platform {PlatformId}", pending.CommandId, kind, platformId);
        }
        catch (OperationCanceledException)
        {
            return EdgeAgentCommandRouterResult.Failure("Edge Agent command timed out or was canceled.");
        }
        catch (InvalidOperationException ex)
        {
            return EdgeAgentCommandRouterResult.Failure(ex.Message);
        }

        var completed = false;
        try
        {
            await foreach (var item in pending.Reader.ReadAllAsync(timeoutCts.Token))
            {
                if (item.ErrorMessage is not null)
                {
                    completed = true;
                    logger.LogWarning("Edge unary command {CommandId} {CommandKind} failed for platform {PlatformId}: {Message}", pending.CommandId, kind, platformId, item.ErrorMessage);
                    return EdgeAgentCommandRouterResult.Failure(item.ErrorMessage);
                }

                if (item.Payload is not null)
                {
                    completed = true;
                    logger.LogDebug("Edge unary command {CommandId} {CommandKind} completed for platform {PlatformId}", pending.CommandId, kind, platformId);
                    return EdgeAgentCommandRouterResult.Success(item.Payload);
                }

                if (item.Completed)
                {
                    completed = true;
                    logger.LogDebug("Edge unary command {CommandId} {CommandKind} completed for platform {PlatformId}", pending.CommandId, kind, platformId);
                    return EdgeAgentCommandRouterResult.Success([]);
                }
            }

            completed = true;
            return EdgeAgentCommandRouterResult.Failure("Edge Agent command completed without a response.");
        }
        catch (OperationCanceledException)
        {
            await CancelPendingAsync(session, pending.CommandId, "Edge Agent command timed out or was canceled.");
            completed = true;
            return EdgeAgentCommandRouterResult.Failure("Edge Agent command timed out or was canceled.");
        }
        finally
        {
            if (completed)
            {
                session.RemovePending(pending.CommandId, failureReason: null);
            }
            else
            {
                await CancelPendingAsync(session, pending.CommandId, "Edge Agent command was canceled.");
            }
        }
    }

    public async IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            yield return EdgeAgentStreamItem.Failure("Edge Agent command payload exceeded the maximum payload size.");
            yield break;
        }

        if (!registry.TryGet(platformId, out var session))
        {
            yield return EdgeAgentStreamItem.Failure("Edge Agent is offline.");
            yield break;
        }

        using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutCts.CancelAfter(timeout);

        EdgePendingCommand? pending = null;
        EdgeAgentStreamItem? startupFailure = null;
        try
        {
            pending = await session.SendCommandAsync(
                MapKind(kind),
                payload,
                timeout,
                correlationId,
                expectsStream: true,
                timeoutCts.Token);
            logger.LogDebug("Started Edge stream command {CommandId} {CommandKind} for platform {PlatformId}", pending.CommandId, kind, platformId);
        }
        catch (OperationCanceledException)
        {
            startupFailure = EdgeAgentStreamItem.Failure("Edge Agent command timed out or was canceled.");
        }
        catch (InvalidOperationException ex)
        {
            startupFailure = EdgeAgentStreamItem.Failure(ex.Message);
        }

        if (startupFailure is not null || pending is null)
        {
            yield return startupFailure ?? EdgeAgentStreamItem.Failure("Failed to start Edge Agent command.");
            yield break;
        }

        var completed = false;
        try
        {
            while (true)
            {
                bool canRead;
                EdgeAgentStreamItem? failure = null;

                try
                {
                    canRead = await pending.Reader.WaitToReadAsync(timeoutCts.Token);
                }
                catch (OperationCanceledException)
                {
                    canRead = false;
                    failure = EdgeAgentStreamItem.Failure("Edge Agent command timed out or was canceled.");
                }

                if (failure is not null)
                {
                    await CancelPendingAsync(session, pending.CommandId, "Edge Agent command timed out or was canceled.");
                    completed = true;
                    yield return failure;
                    yield break;
                }

                if (!canRead)
                {
                    completed = true;
                    yield break;
                }

                while (pending.Reader.TryRead(out var item))
                {
                    if (item.Completed || item.ErrorMessage is not null)
                    {
                        completed = true;
                        if (item.ErrorMessage is not null)
                        {
                            logger.LogWarning("Edge stream command {CommandId} {CommandKind} failed for platform {PlatformId}: {Message}", pending.CommandId, kind, platformId, item.ErrorMessage);
                        }
                        else
                        {
                            logger.LogDebug("Edge stream command {CommandId} {CommandKind} completed for platform {PlatformId}", pending.CommandId, kind, platformId);
                        }
                    }

                    yield return item;
                    if (item.Completed)
                    {
                        yield break;
                    }
                }
            }
        }
        finally
        {
            if (completed)
            {
                session.RemovePending(pending.CommandId, failureReason: null);
            }
            else
            {
                await CancelPendingAsync(session, pending.CommandId, "Edge Agent stream was canceled.");
            }
        }
    }

    public async Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
        Guid platformId,
        EdgeAgentCommandKind kind,
        byte[] payload,
        TimeSpan timeout,
        string? correlationId,
        CancellationToken cancellationToken)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            return Result.Failure<EdgeAgentInteractiveCommand>("Edge Agent command payload exceeded the maximum payload size.");
        }

        if (!registry.TryGet(platformId, out var session))
        {
            return Result.Failure<EdgeAgentInteractiveCommand>("Edge Agent is offline.");
        }

        var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        if (timeout != Timeout.InfiniteTimeSpan)
        {
            timeoutCts.CancelAfter(timeout);
        }

        EdgePendingCommand pending;
        try
        {
            pending = await session.SendCommandAsync(
                MapKind(kind),
                payload,
                timeout,
                correlationId,
                expectsStream: true,
                timeoutCts.Token);
            logger.LogDebug("Started Edge interactive command {CommandId} {CommandKind} for platform {PlatformId}", pending.CommandId, kind, platformId);
        }
        catch (OperationCanceledException)
        {
            timeoutCts.Dispose();
            return Result.Failure<EdgeAgentInteractiveCommand>("Edge Agent command timed out or was canceled.");
        }
        catch (InvalidOperationException ex)
        {
            timeoutCts.Dispose();
            return Result.Failure<EdgeAgentInteractiveCommand>(ex.Message);
        }
        catch
        {
            timeoutCts.Dispose();
            throw;
        }

        return Result.Success(new EdgeAgentInteractiveCommand(
            pending.CommandId,
            ReadInteractiveAsync(session, pending, timeoutCts)));
    }

    public async Task<Result> SendStreamInputAsync(
        Guid platformId,
        string commandId,
        byte[] payload,
        CancellationToken cancellationToken)
    {
        if (!registry.TryGet(platformId, out var session))
        {
            return Result.Failure("Edge Agent is offline.");
        }

        try
        {
            await session.SendStreamInputAsync(commandId, payload, cancellationToken);
            return Result.Success();
        }
        catch (Exception ex) when (ex is InvalidOperationException or OperationCanceledException)
        {
            return Result.Failure(ex.Message);
        }
    }

    public async Task<Result> CancelAsync(
        Guid platformId,
        string commandId,
        string reason,
        CancellationToken cancellationToken)
    {
        if (!registry.TryGet(platformId, out var session))
        {
            return Result.Success();
        }

        try
        {
            await session.CancelCommandAsync(commandId, reason, cancellationToken);
            return Result.Success();
        }
        catch (Exception ex) when (ex is InvalidOperationException or OperationCanceledException)
        {
            return Result.Failure(ex.Message);
        }
    }

    private static async IAsyncEnumerable<EdgeAgentStreamItem> ReadInteractiveAsync(
        EdgeAgentSession session,
        EdgePendingCommand pending,
        CancellationTokenSource timeoutCts)
    {
        var completed = false;
        try
        {
            while (true)
            {
                bool canRead;
                EdgeAgentStreamItem? failure = null;

                try
                {
                    canRead = await pending.Reader.WaitToReadAsync(timeoutCts.Token);
                }
                catch (OperationCanceledException)
                {
                    canRead = false;
                    failure = EdgeAgentStreamItem.Failure("Edge Agent command timed out or was canceled.");
                }

                if (failure is not null)
                {
                    await CancelPendingAsync(session, pending.CommandId, "Edge Agent command timed out or was canceled.");
                    completed = true;
                    yield return failure;
                    yield break;
                }

                if (!canRead)
                {
                    completed = true;
                    yield break;
                }

                while (pending.Reader.TryRead(out var item))
                {
                    if (item.Completed || item.ErrorMessage is not null)
                    {
                        completed = true;
                    }

                    yield return item;
                    if (item.Completed)
                    {
                        yield break;
                    }
                }
            }
        }
        finally
        {
            timeoutCts.Dispose();
            if (completed)
            {
                session.RemovePending(pending.CommandId, failureReason: null);
            }
            else
            {
                await CancelPendingAsync(session, pending.CommandId, "Edge Agent interactive command was canceled.");
            }
        }
    }

    private static async Task CancelPendingAsync(EdgeAgentSession session, string commandId, string reason)
    {
        try
        {
            await session.CancelCommandAsync(commandId, reason, CancellationToken.None);
        }
        catch (InvalidOperationException)
        {
            session.RemovePending(commandId, reason);
        }
        catch (OperationCanceledException)
        {
            session.RemovePending(commandId, reason);
        }
    }

    private static ProtoEdgeCommandKind MapKind(EdgeAgentCommandKind kind)
        => kind switch
        {
            EdgeAgentCommandKind.PlatformCheckHealth => ProtoEdgeCommandKind.PlatformCheckHealth,
            EdgeAgentCommandKind.PlatformGetInfo => ProtoEdgeCommandKind.PlatformGetInfo,
            EdgeAgentCommandKind.PlatformStatsStream => ProtoEdgeCommandKind.PlatformStatsStream,
            EdgeAgentCommandKind.PlatformDaemonEventsStream => ProtoEdgeCommandKind.PlatformDaemonEventsStream,
            EdgeAgentCommandKind.PlatformPrune => ProtoEdgeCommandKind.PlatformPrune,
            EdgeAgentCommandKind.ContainerList => ProtoEdgeCommandKind.ContainerList,
            EdgeAgentCommandKind.ContainerLogsStream => ProtoEdgeCommandKind.ContainerLogsStream,
            EdgeAgentCommandKind.ContainerInspect => ProtoEdgeCommandKind.ContainerInspect,
            EdgeAgentCommandKind.ContainerCreate => ProtoEdgeCommandKind.ContainerCreate,
            EdgeAgentCommandKind.ContainerStart => ProtoEdgeCommandKind.ContainerStart,
            EdgeAgentCommandKind.ContainerStop => ProtoEdgeCommandKind.ContainerStop,
            EdgeAgentCommandKind.ContainerPause => ProtoEdgeCommandKind.ContainerPause,
            EdgeAgentCommandKind.ContainerUnpause => ProtoEdgeCommandKind.ContainerUnpause,
            EdgeAgentCommandKind.ContainerRestart => ProtoEdgeCommandKind.ContainerRestart,
            EdgeAgentCommandKind.ContainerDelete => ProtoEdgeCommandKind.ContainerDelete,
            EdgeAgentCommandKind.ContainerStatsStream => ProtoEdgeCommandKind.ContainerStatsStream,
            EdgeAgentCommandKind.ContainersStatsStream => ProtoEdgeCommandKind.ContainersStatsStream,
            EdgeAgentCommandKind.ContainerExec => ProtoEdgeCommandKind.ContainerExec,
            EdgeAgentCommandKind.ContainerExecBinary => ProtoEdgeCommandKind.ContainerExecBinary,
            EdgeAgentCommandKind.ImageGet => ProtoEdgeCommandKind.ImageGet,
            EdgeAgentCommandKind.ImageList => ProtoEdgeCommandKind.ImageList,
            EdgeAgentCommandKind.ImageInspect => ProtoEdgeCommandKind.ImageInspect,
            EdgeAgentCommandKind.ImageDelete => ProtoEdgeCommandKind.ImageDelete,
            EdgeAgentCommandKind.ImageHistory => ProtoEdgeCommandKind.ImageHistory,
            EdgeAgentCommandKind.ImageExposedPorts => ProtoEdgeCommandKind.ImageExposedPorts,
            EdgeAgentCommandKind.ImageDistributionInspect => ProtoEdgeCommandKind.ImageDistributionInspect,
            EdgeAgentCommandKind.ImagePullStream => ProtoEdgeCommandKind.ImagePullStream,
            EdgeAgentCommandKind.ImageBuildStream => ProtoEdgeCommandKind.ImageBuildStream,
            EdgeAgentCommandKind.ImagePushStream => ProtoEdgeCommandKind.ImagePushStream,
            EdgeAgentCommandKind.VolumeList => ProtoEdgeCommandKind.VolumeList,
            EdgeAgentCommandKind.VolumeInspect => ProtoEdgeCommandKind.VolumeInspect,
            EdgeAgentCommandKind.VolumeCreate => ProtoEdgeCommandKind.VolumeCreate,
            EdgeAgentCommandKind.VolumeDelete => ProtoEdgeCommandKind.VolumeDelete,
            EdgeAgentCommandKind.NetworkList => ProtoEdgeCommandKind.NetworkList,
            EdgeAgentCommandKind.NetworkInspect => ProtoEdgeCommandKind.NetworkInspect,
            EdgeAgentCommandKind.NetworkCreate => ProtoEdgeCommandKind.NetworkCreate,
            EdgeAgentCommandKind.NetworkDelete => ProtoEdgeCommandKind.NetworkDelete,
            EdgeAgentCommandKind.StackApplyStream => ProtoEdgeCommandKind.StackApplyStream,
            EdgeAgentCommandKind.DeploymentApply => ProtoEdgeCommandKind.DeploymentApply,
            _ => ProtoEdgeCommandKind.Unspecified
        };
}
