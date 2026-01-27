using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;
public interface IExecSessionManager : IStreamGroupManager
{
    Task SendInputAsync(string groupId, byte[] data, CancellationToken ct);
    Task ResizeAsync(string groupId, int cols, int rows, CancellationToken ct);
}

internal sealed class ExecSessionManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ExecSessionManager> logger,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IPlatformContainerCache platformContainerCache)
    : BaseStreamManager<ExecStreamContext>, IExecSessionManager
{
    protected override void OnSubscriberAdded(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var ctx))
            return;

        var containerId = GetNormalizedIdFromGroup(groupId.AsSpan());
        if (string.IsNullOrEmpty(containerId))
            return;

        if (ctx.TryStart())
            Run(ctx, containerId);
    }

    private void Run(ExecStreamContext ctx, string containerId)
    {
        ctx.StreamTask = Task.Run(() => StreamExecAsync(ctx, containerId), ctx.Cancellation.Token);
    }

    private async Task StreamExecAsync(ExecStreamContext ctx, string containerId)
    {
        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
            return;

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var token = ctx.Cancellation.Token;

        try
        {
            var session = await connector.ExecAsync(containerId, "/bin/bash", token);
            ctx.Session = session;

            await foreach (var chunk in session.Output.WithCancellation(token))
            {
                await dispatcher.SendExecOutput(containerId, chunk.ToArray());
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
                await ctx.Session.DisposeAsync();
        }
    }

    public async Task SendInputAsync(string groupId, byte[] data, CancellationToken ct)
    {
        if (!streams.TryGetValue(groupId, out var ctx)) return;
        if (ctx.Session == null) return;

        await ctx.Session.SendAsync(data, ct);
    }

    public async Task ResizeAsync(string groupId, int cols, int rows, CancellationToken ct)
    {
        if (!streams.TryGetValue(groupId, out var ctx)) return;
        if (ctx.Session == null) return;

        await ctx.Session.ResizeAsync(cols, rows, ct);
    }
}