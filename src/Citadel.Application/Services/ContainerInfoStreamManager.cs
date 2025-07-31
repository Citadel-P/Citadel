using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Configs;
using Application.Services.Abstractions;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services;

/// <summary>
/// Manages and start/stop a stream based on number of active subscribers
/// </summary>
public interface IContainerInfoStreamManager
{
    void AddSubscriber(string containerId, string connectionId);
    void RemoveSubscriber(string containerId, string connectionId);
}

public class ContainerInfoStreamManager(
    IOptions<JobConfiguration> options,
    IContainerInfoHubDispatcher dispatcher, 
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerInfoStreamManager> logger) : IContainerInfoStreamManager
{
    private readonly ConcurrentDictionary<string, ContainerStreamContext> streams = new();

    public void AddSubscriber(string containerId, string connectionId)
    {
        var context = streams.GetOrAdd(containerId, _ =>
        {
            var ctx = new ContainerStreamContext();
            Task.Run(() => PollDockerStats(containerId, ctx));
            Task.Run(() => BroadcastStats(ctx));
            return ctx;
        });

        context.AddSubscriber(connectionId);
    }

    public void RemoveSubscriber(string containerId, string connectionId)
    {
        if (!streams.TryGetValue(containerId, out var context))
            return;

        context.RemoveSubscriber(connectionId);

        if (context.IsEmpty)
        {
            context.Cancellation.Cancel();
            streams.TryRemove(containerId, out _);
        }
    }

    private async Task PollDockerStats(string containerId, ContainerStreamContext ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;
        
        if (!platformContainerCache.TryGetPlatformByContainerId(containerId, out var platformInfo))
        {
            logger.LogError("No platform found for container ID {ContainerId}", containerId);
            return;
        }

        try
        {
            while (!token.IsCancellationRequested)
            {
                await foreach(var container in connectorFactory.GetConnector(platformInfo.Type).StreamContainerStatsAsync(new StreamContainerStatsCommand(containerId, platformInfo.PlatformAddress, options.Value.ContainersInfoInterval * 1000), token))
                {
                    await writer.WriteAsync(container, token);
                }
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            writer.Complete();
        }
    }

    private async Task BroadcastStats(ContainerStreamContext ctx)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;
        
        try
        {
            await foreach (var container in reader.ReadAllAsync(token))
            {
                using var _ = container;
                await dispatcher.SendContainerInfo(container.Value, token);
            }
        }
        catch (OperationCanceledException) { }
    }

}
internal sealed class ContainerStreamContext
{
    public CancellationTokenSource Cancellation { get; } = new();
    public Channel<PooledHandle<DockerContainer>> Channel { get; } 
        = System.Threading.Channels.Channel.CreateBounded<PooledHandle<DockerContainer>>(ApplicationModule.ChannelDefaultOptions());
    
    private readonly Lock @lock = new();
    private readonly HashSet<string> subscribers = [];

    public void AddSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Add(connectionId);
        }
    }

    public void RemoveSubscriber(string connectionId)
    {
        lock (@lock)
        {
            subscribers.Remove(connectionId);
        }
    }

    public bool IsEmpty
    {
        get
        {
            lock (@lock)
                return subscribers.Count == 0;
        }
    }
}