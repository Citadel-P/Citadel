using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IDockerDaemonStreamManager : IStreamGroupManager
{
    Task SendContainerEvent(Container container, string @event);
    Task SendImageEvent(Image image, string @event);
}

internal class DockerDaemonStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IDockerDaemonStreamManager
{
    public Task SendContainerEvent(Container container, string @event)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainerEvent(container, @event);
    }

    public Task SendImageEvent(Image image, string @event)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendImageEvent(image, @event);
    }
}
