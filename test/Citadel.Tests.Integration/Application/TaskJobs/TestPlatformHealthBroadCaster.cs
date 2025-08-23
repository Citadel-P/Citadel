using System.Threading.Channels;
using Application.Services;
using Application.TaskJobs;

namespace Tests.Integration.Application.TaskJobs;

internal sealed class TestPlatformHealthBroadCaster : IPlatformHealthBroadCaster
{
    private readonly Channel<PlatformHealth> _channel;

    public TestPlatformHealthBroadCaster()
    {
        _channel = Channel.CreateUnbounded<PlatformHealth>();
    }

    public ChannelReader<PlatformHealth> AddSubscriber() => _channel.Reader;

    public void Complete() => _channel.Writer.TryComplete();

    public ValueTask PublishAsync(PlatformHealth platformHealth, CancellationToken cancellationToken)
        => _channel.Writer.WriteAsync(platformHealth, cancellationToken);

}