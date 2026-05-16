using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Grpc.Core;
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
    public async Task StartExecProcess(string containerId, string sessionId, string shell, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(containerId))
            return;

        var normalized = NormalizeDockerId(containerId);

        var ctx = streams.GetOrAdd(sessionId, _ => new ExecStreamContext());

        if (string.IsNullOrEmpty(normalized) || string.IsNullOrEmpty(sessionId))
            return;

        if (!ctx.TryStart()) return;

        ctx.StreamTask = Task.Run(
           () => StreamExecAsync(ctx, normalized, sessionId, shell, ct), ctx.Cancellation.Token);
    }

    public async Task SendInputAsync(string containerId, string sessionId, byte[] data, CancellationToken ct)
    {
        if (!streams.TryGetValue(sessionId, out var ctx)) return;
        if (ctx.Session == null) return;

        await ctx.Session.SendAsync(data, ct);
    }

    public async Task ResizeAsync(string containerId, string sessionId, int cols, int rows, CancellationToken ct)
    {
        if (!streams.TryGetValue(sessionId, out var ctx)) return;

        ctx.LatestCols = cols;
        ctx.LatestRows = rows;
        if (ctx.Session == null) return;

        await ctx.Session.ResizeAsync(cols, rows, ct);
    }

    private async Task StreamExecAsync(
        ExecStreamContext ctx,
        string containerId,
        string sessionId,
        string shell,
        CancellationToken callerToken)
    {
        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
            return;

        using var linkedCts =
            CancellationTokenSource.CreateLinkedTokenSource(ctx.Cancellation.Token, callerToken);

        var token = linkedCts.Token;
        var connector = connectorFactory.GetConnector(platform.ConnectorType);

        try
        {
            var shellPath = shell == "sh" ? "/bin/sh" : "/bin/bash";

            var session = await connector.ExecAsync(platform.Address, containerId, shellPath, token);
            ctx.Session = session;

            if (ctx.LatestCols > 0 && ctx.LatestRows > 0)
            {
                await session.ResizeAsync(ctx.LatestCols, ctx.LatestRows, token);
            }

            await foreach (var chunk in session.Output.WithCancellation(token))
            {
                await dispatcher.SendExecOutput(containerId, sessionId, chunk.ToArray());
            }
        }
        catch (OperationCanceledException) { }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
            logger.LogDebug("Exec stream cancelled for {ContainerId}", containerId);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Exec stream failed for {ContainerId}", containerId);
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
