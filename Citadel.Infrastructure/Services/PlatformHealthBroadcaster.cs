using System.Threading.Channels;
using Infrastructure.TaskJobs;

namespace Infrastructure.Services;

/// <summary>
/// Broadcaster for platform health events.  
/// </summary>
public interface IPlatformHealthBroadCaster
{
    ChannelReader<PlatformHealth> Register();
    Task BroadcastAsync(PlatformHealth evt, CancellationToken cancellationToken);
    void Complete();
}

internal class PlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly List<Channel<PlatformHealth>> _channels = [];

    public ChannelReader<PlatformHealth> Register()
    {
        var channel = Channel.CreateBounded<PlatformHealth>(InfrastructureModule.ChannelDefaultOptions());
        _channels.Add(channel);
        return channel.Reader;
    }

    public async Task BroadcastAsync(PlatformHealth evt, CancellationToken cancellationToken)
    {
        foreach (var ch in _channels)
        {
            await ch.Writer.WriteAsync(evt, cancellationToken);
        }
    }

    public void Complete()
    {
        foreach (var ch in _channels)
            ch.Writer.TryComplete();
    }
}
