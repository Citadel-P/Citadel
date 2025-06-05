using System.Threading.Channels;
using Infrastructure.TaskJobs;

namespace Infrastructure.Services;

/// <summary>
/// Broadcaster for platform health events.  
/// </summary>
internal interface IPlatformHealthBroadCaster
{
    ChannelReader<PlatformHealth> Register();
    Task BroadcastAsync(PlatformHealth evt, CancellationToken token);
    void Complete();
}

internal class PlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly List<Channel<PlatformHealth>> _channels = [];

    public ChannelReader<PlatformHealth> Register()
    {
        var channel = Channel.CreateUnbounded<PlatformHealth>();
        _channels.Add(channel);
        return channel.Reader;
    }

    public async Task BroadcastAsync(PlatformHealth evt, CancellationToken token)
    {
        foreach (var ch in _channels)
        {
            await ch.Writer.WriteAsync(evt, token);
        }
    }

    public void Complete()
    {
        foreach (var ch in _channels)
            ch.Writer.TryComplete();
    }
}
