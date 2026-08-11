using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Grpc.Core;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

public interface IExecSessionManager : IStreamGroupManager
{
    Task StartExecProcess(string containerId, string sessionId, string shell, CancellationToken ct);
    Task SendInputAsync(string containerId, string sessionId, byte[] data, CancellationToken ct);
    Task ResizeAsync(string containerId, string sessionId, int cols, int rows, CancellationToken ct);
    Task StartSwarmTaskExecProcess(
        Platform platform,
        string taskId,
        string dockerNodeId,
        string dockerContainerId,
        string sessionId,
        string shell,
        CancellationToken ct);
    Task SendSwarmTaskInputAsync(Guid platformId, string taskId, string sessionId, byte[] data, CancellationToken ct);
    Task ResizeSwarmTaskAsync(Guid platformId, string taskId, string sessionId, int cols, int rows, CancellationToken ct);
}

internal sealed class ExecSessionManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ExecSessionManager> logger,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IServiceScopeFactory scopeFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector)
    : BaseStreamManager<ExecStreamContext>, IExecSessionManager
{
    public Task StartExecProcess(string containerId, string sessionId, string shell, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(containerId) || string.IsNullOrWhiteSpace(sessionId))
            return Task.CompletedTask;

        var containerReference = NormalizeContainerReference(containerId);
        if (string.IsNullOrEmpty(containerReference))
            return Task.CompletedTask;

        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerReference, sessionId);
        TryUseStream(groupId, ctx =>
        {
            if (ctx.TryStart())
                ctx.StreamTask = StreamExecAsync(ctx, containerReference, sessionId, shell, ct);
        });

        return Task.CompletedTask;
    }

    public async Task SendInputAsync(string containerId, string sessionId, byte[] data, CancellationToken ct)
    {
        var containerReference = NormalizeContainerReference(containerId);
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerReference, sessionId);
        IExecSession? session = null;
        TryUseStream(groupId, ctx => session = ctx.Session);

        if (session is not null)
            await session.SendAsync(data, ct);
    }

    public async Task ResizeAsync(string containerId, string sessionId, int cols, int rows, CancellationToken ct)
    {
        var containerReference = NormalizeContainerReference(containerId);
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerReference, sessionId);
        IExecSession? session = null;
        TryUseStream(groupId, ctx =>
        {
            ctx.LatestCols = cols;
            ctx.LatestRows = rows;
            session = ctx.Session;
        });

        if (session is not null)
            await session.ResizeAsync(cols, rows, ct);
    }

    public Task StartSwarmTaskExecProcess(
        Platform platform,
        string taskId,
        string dockerNodeId,
        string dockerContainerId,
        string sessionId,
        string shell,
        CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(taskId)
            || string.IsNullOrWhiteSpace(dockerNodeId)
            || string.IsNullOrWhiteSpace(dockerContainerId)
            || string.IsNullOrWhiteSpace(sessionId))
        {
            return Task.CompletedTask;
        }

        var groupId = Constants.WellKnownSignalRGroups.SwarmTaskExecGroup(platform.Id, taskId, sessionId);
        TryUseStream(groupId, context =>
        {
            if (context.TryStart())
            {
                context.StreamTask = StreamSwarmTaskExecAsync(
                    context,
                    platform,
                    taskId,
                    dockerNodeId,
                    dockerContainerId,
                    sessionId,
                    shell,
                    ct);
            }
        });
        return Task.CompletedTask;
    }

    public async Task SendSwarmTaskInputAsync(
        Guid platformId,
        string taskId,
        string sessionId,
        byte[] data,
        CancellationToken ct)
    {
        var groupId = Constants.WellKnownSignalRGroups.SwarmTaskExecGroup(platformId, taskId, sessionId);
        IExecSession? session = null;
        TryUseStream(groupId, context => session = context.Session);
        if (session is not null)
            await session.SendAsync(data, ct);
    }

    public async Task ResizeSwarmTaskAsync(
        Guid platformId,
        string taskId,
        string sessionId,
        int cols,
        int rows,
        CancellationToken ct)
    {
        var groupId = Constants.WellKnownSignalRGroups.SwarmTaskExecGroup(platformId, taskId, sessionId);
        IExecSession? session = null;
        TryUseStream(groupId, context =>
        {
            context.LatestCols = cols;
            context.LatestRows = rows;
            session = context.Session;
        });
        if (session is not null)
            await session.ResizeAsync(cols, rows, ct);
    }

    private async Task StreamExecAsync(
        ExecStreamContext ctx,
        string containerReference,
        string sessionId,
        string shell,
        CancellationToken callerToken)
    {
        try
        {
            var target = await ResolveTargetAsync(containerReference, callerToken);
            if (target is null)
                return;

            using var linkedCts =
                CancellationTokenSource.CreateLinkedTokenSource(ctx.Cancellation.Token, callerToken);

            var token = linkedCts.Token;
            var shellPath = shell == "sh" ? "/bin/sh" : "/bin/bash";
            var session = target.DockerNodeId is not null
                ? await swarmNodeRuntimeConnector.ExecAsync(
                    target.Platform,
                    target.DockerNodeId,
                    target.DockerContainerId,
                    shellPath,
                    token)
                : await connectorFactory.GetConnector(target.Platform.ConnectorType).ExecAsync(
                    target.Platform.Address,
                    target.DockerContainerId,
                    shellPath,
                    token);
            ctx.Session = session;

            if (ctx.LatestCols > 0 && ctx.LatestRows > 0)
            {
                await session.ResizeAsync(ctx.LatestCols, ctx.LatestRows, token);
            }

            await foreach (var chunk in session.Output.WithCancellation(token))
            {
                await dispatcher.SendExecOutput(containerReference, sessionId, chunk.ToArray());
            }
        }
        catch (OperationCanceledException) { }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
            logger.LogDebug("Exec stream cancelled for {ContainerReference}", containerReference);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Exec stream failed for {ContainerReference}", containerReference);
        }
        finally
        {
            if (ctx.Session != null)
            {
                await ctx.Session.DisposeAsync();
                ctx.Session = null;
            }

            ctx.ResetStarted();
        }
    }

    private async Task<ContainerExecTarget?> ResolveTargetAsync(
        string containerReference,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = Guid.TryParse(containerReference, out var resourceId)
            ? await uow.Containers.GetByIdAsync(resourceId, cancellationToken)
            : await uow.Containers.GetByIdAsync(containerReference, cancellationToken);
        if (container is null)
            return null;

        var platform = await uow.Platforms.GetByIdAsync(container.PlatformId, cancellationToken);
        return platform is null
            ? null
            : new ContainerExecTarget(platform, container.DockerNodeId, container.DockerContainerId);
    }

    private async Task StreamSwarmTaskExecAsync(
        ExecStreamContext context,
        Platform platform,
        string taskId,
        string dockerNodeId,
        string dockerContainerId,
        string sessionId,
        string shell,
        CancellationToken callerToken)
    {
        try
        {
            using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(
                context.Cancellation.Token,
                callerToken);
            var token = linkedCts.Token;
            var shellPath = shell == "sh" ? "/bin/sh" : "/bin/bash";
            var session = await swarmNodeRuntimeConnector.ExecAsync(
                platform,
                dockerNodeId,
                dockerContainerId,
                shellPath,
                token);
            context.Session = session;

            if (context.LatestCols > 0 && context.LatestRows > 0)
                await session.ResizeAsync(context.LatestCols, context.LatestRows, token);

            await foreach (var chunk in session.Output.WithCancellation(token))
                await dispatcher.SendSwarmTaskExecOutput(platform.Id, taskId, sessionId, chunk.ToArray());
        }
        catch (OperationCanceledException) { }
        catch (RpcException exception) when (exception.StatusCode == StatusCode.Cancelled)
        {
            logger.LogDebug("Swarm Task exec stream cancelled for {TaskId} on Node {DockerNodeId}", taskId, dockerNodeId);
        }
        catch (Exception exception)
        {
            logger.LogError(exception, "Swarm Task exec stream failed for {TaskId} on Node {DockerNodeId}", taskId, dockerNodeId);
        }
        finally
        {
            if (context.Session is not null)
            {
                await context.Session.DisposeAsync();
                context.Session = null;
            }
            context.ResetStarted();
        }
    }

    private sealed record ContainerExecTarget(
        Platform Platform,
        string? DockerNodeId,
        string DockerContainerId);
}
