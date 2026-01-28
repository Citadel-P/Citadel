using System.Text;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

public interface IExecSessionManager : IStreamGroupManager
{
    Task StartExecProcess(string groupId, string shell, CancellationToken ct);
    Task SendInputAsync(string groupId, byte[] data, CancellationToken ct);
    Task ResizeAsync(string groupId, int cols, int rows, string shell, CancellationToken ct);
}

internal sealed class ExecSessionManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ExecSessionManager> logger,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IPlatformContainerCache platformContainerCache)
    : BaseStreamManager<ExecStreamContext>, IExecSessionManager
{
    public Task StartExecProcess(string groupId, string shell, CancellationToken ct)
    {
        var ctx = streams.GetOrAdd(groupId, _ => new ExecStreamContext());
        ctx.Shell = shell;

        TryEnsureStarted(groupId, ctx, ct);
        return Task.CompletedTask;
    }

    public async Task SendInputAsync(string groupId, byte[] data, CancellationToken ct)
    {
        if (!streams.TryGetValue(groupId, out var ctx)) return;
        if (ctx.Session == null) return;

        await ctx.Session.SendAsync(data, ct);
    }

    public Task ResizeAsync(string groupId, int cols, int rows, string shell, CancellationToken ct)
    {
        var ctx = streams.GetOrAdd(groupId, _ => new ExecStreamContext());

        ctx.Shell = shell;
        ctx.LatestCols = cols;
        ctx.LatestRows = rows;

        if (ctx.Session != null)
        {
            return ctx.Session.ResizeAsync(cols, rows, ct);
        }

        // ensure start if resize arrived first
        TryEnsureStarted(groupId, ctx, ct);
        return Task.CompletedTask;
    }

    private void TryEnsureStarted(string groupId, ExecStreamContext ctx, CancellationToken ct)
    {
        var (containerId, sessionId) = GetContainerIdFromGroup(groupId);
        if (string.IsNullOrEmpty(containerId) || string.IsNullOrEmpty(sessionId))
            return;

        if (!ctx.TryStart()) return;

        Run(ctx, containerId, sessionId, ctx.Shell ?? "bash", ct);
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

            var session = await connector.ExecAsync(containerId, shellPath, token);
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

    private void Run(
        ExecStreamContext ctx,
        string containerId,
        string sessionId,
        string shell,
        CancellationToken ct)
    {
        ctx.StreamTask = Task.Run(
            () => StreamExecAsync(ctx, containerId, sessionId, shell, ct),
            ctx.Cancellation.Token);
    }

    private static (string, string) GetContainerIdFromGroup(string groupId)
    {
        // format: container-exec:{containerId}:{sessionId}
        var parts = groupId.Split(':', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length < 3) return (string.Empty, string.Empty);
        return (NormalizeDockerId(parts[1]), parts[2]);
    }
}
