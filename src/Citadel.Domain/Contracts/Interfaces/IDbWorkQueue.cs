using System.Threading.Channels;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// A single-writer DB queue
/// </summary>
public interface IDbWorkQueue
{
    ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken);
    ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken);
    ChannelReader<IDbWorkItem> Reader { get; }
}

public interface IDbWorkItem
{
    Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken);
}
