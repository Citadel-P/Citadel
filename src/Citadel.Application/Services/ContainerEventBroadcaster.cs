using System.Threading.Channels;

namespace Application.Services;

internal interface IContainerEventBroadcaster
{
    ChannelReader<ContainerEvent> AddSubscriber();
    ValueTask PublishAsync(ContainerEvent ev, CancellationToken cancellationToken = default);
}

internal sealed class ContainerEventBroadcaster : MulticastChannel<ContainerEvent>, IContainerEventBroadcaster
{
    
}

internal record ContainerEvent(Guid PlatformId, string ContainerId, string Action);