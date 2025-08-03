using Domain.Entities;

namespace Application.Services.Abstractions;

public interface IDockerDaemonHubDispatcher
{
    Task SendContainerEvent(Container container, string @event);
}