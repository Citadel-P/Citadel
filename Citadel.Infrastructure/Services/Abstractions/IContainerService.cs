using Contracts.Broker.Models;

namespace Infrastructure.Services.Abstractions;

public interface IContainerService
{
    /// <summary>
    /// Handles the reception of <see cref="ContainersInfoMessage"/> from the broker
    /// </summary>
    Task OnContainersInfoMessage(ContainerListMessage message, CancellationToken cancellationToken);

    /// <summary>
    /// Handles the reception of <see cref="ContainerLogMessage"/> from the broker
    /// </summary>
    Task OnContainerLogsMessage(ContainerLogMessage message, CancellationToken cancellationToken);

    /// <summary>
    /// Handles the reception of <see cref="ContainerEventMessage"/> from the broker
    /// </summary>
    Task OnContainerEventMessage(ContainerEventMessage message, CancellationToken cancellationToken);
}