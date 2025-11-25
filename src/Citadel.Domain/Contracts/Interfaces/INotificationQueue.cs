using System.Threading.Channels;

namespace Domain.Contracts.Interfaces;

public interface INotificationQueue
{
    ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct);
    ChannelReader<INotificationWorkItem> Reader { get; }
}

public interface INotificationWorkItem
{
    Task ExecuteAsync(CancellationToken cancellationToken);
}