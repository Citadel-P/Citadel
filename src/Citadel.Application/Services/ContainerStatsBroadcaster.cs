using System.Threading.Channels;
using Domain.Entities;

namespace Application.Services;

internal interface IContainerStatsBroadcaster
{
    bool HasSubscribers { get; }
    ChannelReader<ContainerStatsSnapshot> AddSubscriber();
    void RemoveSubscriber(ChannelReader<ContainerStatsSnapshot> reader);
    ValueTask PublishAsync(ContainerStatsSnapshot snapshot, CancellationToken cancellationToken);
}

internal sealed class ContainerStatsBroadcaster
    : MulticastChannel<ContainerStatsSnapshot>, IContainerStatsBroadcaster
{
}

internal sealed record ContainerStatsSnapshot(
    Guid PlatformId,
    IReadOnlyList<ContainerStat> Stats);
