using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IDockerDaemonStreamManager : IStreamGroupManager
{
    Task SendContainerEvent(Container container, string @event);
}

internal class DockerDaemonStreamManager(IDockerHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IDockerDaemonStreamManager
{
    public Task SendContainerEvent(Container container, string @event)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainerEvent(container, @event);
    }
}
