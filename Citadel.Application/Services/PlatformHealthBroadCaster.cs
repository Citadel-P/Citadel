using System.Threading.Channels;
using Application.TaskJobs;

namespace Application.Services;

/// <summary>
/// Broadcaster for platform health events.  
/// </summary>
internal interface IPlatformHealthBroadCaster
{
    void Complete();
    ChannelReader<PlatformHealth> Register();
    Task BroadcastAsync(PlatformHealth platformHealth, CancellationToken cancellationToken);
}

internal class PlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly List<Channel<PlatformHealth>> _channels = [];

    public ChannelReader<PlatformHealth> Register()
    {
        var channel = Channel.CreateBounded<PlatformHealth>(ApplicationModule.ChannelDefaultOptions());
        _channels.Add(channel);
        return channel.Reader;
    }

    public async Task BroadcastAsync(PlatformHealth platformHealth, CancellationToken cancellationToken)
    {
        foreach (var ch in _channels)
        {
            await ch.Writer.WriteAsync(platformHealth, cancellationToken);
        }
    }

    public void Complete()
    {
        foreach (var ch in _channels)
            ch.Writer.TryComplete();
    }
}
