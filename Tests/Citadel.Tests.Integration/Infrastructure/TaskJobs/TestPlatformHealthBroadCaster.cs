using System.Threading.Channels;
using Infrastructure.Services;
using Infrastructure.TaskJobs;

namespace Tests.Integration.Infrastructure.TaskJobs;

internal sealed class TestPlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly Channel<PlatformHealth> _channel;

    public TestPlatformHealthBroadCaster()
    {
        _channel = Channel.CreateUnbounded<PlatformHealth>();
    }

    public ChannelReader<PlatformHealth> Register() => _channel.Reader;

    public Task BroadcastAsync(PlatformHealth evt, CancellationToken cancellationToken)
        => _channel.Writer.WriteAsync(evt, cancellationToken).AsTask();

    public void Complete() => _channel.Writer.TryComplete();
}