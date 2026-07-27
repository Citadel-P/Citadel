using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Grpc.Core;
using Hosting.Common;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

public interface IExecSessionManager : IStreamGroupManager
{
    Task StartExecProcess(string containerId, string sessionId, string shell, CancellationToken ct);
    Task SendInputAsync(string containerId, string sessionId, byte[] data, CancellationToken ct);
    Task ResizeAsync(string containerId, string sessionId, int cols, int rows, CancellationToken ct);
}

internal sealed class ExecSessionManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ExecSessionManager> logger,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IPlatformContainerCache platformContainerCache)
    : BaseStreamManager<ExecStreamContext>, IExecSessionManager
{
    public Task StartExecProcess(string containerId, string sessionId, string shell, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(containerId) || string.IsNullOrWhiteSpace(sessionId))
            return Task.CompletedTask;

        var normalized = NormalizeDockerId(containerId);
        if (string.IsNullOrEmpty(normalized))
            return Task.CompletedTask;

        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, sessionId);
        TryUseStream(groupId, ctx =>
        {
            if (ctx.TryStart())
                ctx.StreamTask = StreamExecAsync(ctx, normalized, containerId, sessionId, shell, ct);
        });

        return Task.CompletedTask;
    }

    public async Task SendInputAsync(string containerId, string sessionId, byte[] data, CancellationToken ct)
    {
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, sessionId);
        IExecSession? session = null;
        TryUseStream(groupId, ctx => session = ctx.Session);

        if (session is not null)
            await session.SendAsync(data, ct);
    }

    public async Task ResizeAsync(string containerId, string sessionId, int cols, int rows, CancellationToken ct)
    {
        var groupId = Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, sessionId);
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

    private async Task StreamExecAsync(
        ExecStreamContext ctx,
        string normalizedContainerId,
        string groupContainerId,
        string sessionId,
        string shell,
        CancellationToken callerToken)
    {
        try
        {
            if (!platformContainerCache.TryGetPlatformWithContainer(normalizedContainerId, out var platform))
                return;

            using var linkedCts =
                CancellationTokenSource.CreateLinkedTokenSource(ctx.Cancellation.Token, callerToken);

            var token = linkedCts.Token;
            var connector = connectorFactory.GetConnector(platform.ConnectorType);
            var shellPath = shell == "sh" ? "/bin/sh" : "/bin/bash";

            var session = await connector.ExecAsync(platform.Address, normalizedContainerId, shellPath, token);
            ctx.Session = session;

            if (ctx.LatestCols > 0 && ctx.LatestRows > 0)
            {
                await session.ResizeAsync(ctx.LatestCols, ctx.LatestRows, token);
            }

            await foreach (var chunk in session.Output.WithCancellation(token))
            {
                await dispatcher.SendExecOutput(groupContainerId, sessionId, chunk.ToArray());
            }
        }
        catch (OperationCanceledException) { }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
            logger.LogDebug("Exec stream cancelled for {ContainerId}", normalizedContainerId);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Exec stream failed for {ContainerId}", normalizedContainerId);
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
}
