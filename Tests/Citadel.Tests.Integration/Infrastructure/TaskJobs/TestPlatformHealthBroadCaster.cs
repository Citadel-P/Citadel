using System.Threading.Channels;
using Application.Services;
using Application.TaskJobs;

namespace Tests.Integration.Infrastructure.TaskJobs;

internal sealed class TestPlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly Channel<PlatformHealth> _channel;

    public TestPlatformHealthBroadCaster()
    {
        _channel = Channel.CreateUnbounded<PlatformHealth>();
    }

    public ChannelReader<PlatformHealth> Register() => _channel.Reader;

    public Task BroadcastAsync(PlatformHealth platformHealth, CancellationToken cancellationToken)
        => _channel.Writer.WriteAsync(platformHealth, cancellationToken).AsTask();

    public void Complete() => _channel.Writer.TryComplete();
}