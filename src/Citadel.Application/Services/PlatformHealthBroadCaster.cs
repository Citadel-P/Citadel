using System.Threading.Channels;
using Application.TaskJobs;

namespace Application.Services;

/// <summary>
/// Broadcaster for platform health events.  
/// </summary>
internal interface IPlatformHealthBroadCaster
{
    void Complete();
    ChannelReader<PlatformHealth> AddSubscriber();
    void RemoveSubscriber(ChannelReader<PlatformHealth> reader);
    ValueTask PublishAsync(PlatformHealth platformHealth, CancellationToken cancellationToken);
}

internal class PlatformHealthBroadCaster : MulticastChannel<PlatformHealth>, IPlatformHealthBroadCaster
{
    
}
