using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Identity;
using Google.Protobuf.WellKnownTypes;

namespace Application.Services.SignalR;

internal interface IDockerDaemonStreamManager : IStreamGroupManager
{
    Task SendContainerEvent(Container container, string @event);
    Task SendImageEvent(Image image, string @event);
    Task SendVolumeEvent(DockerVolumeResult? volume, string @event, string actorId, Guid platformId);
    Task SendNetworkEvent(DockerNetworkResult? network, string @event, string actorId, Guid platformId);
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

    public Task SendNetworkEvent(DockerNetworkResult? network, string @event, string actorId, Guid platformId)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendNetworkEvent(network, @event, actorId, platformId);
    }

    public Task SendVolumeEvent(DockerVolumeResult? volume, string @event, string actorId, Guid platformId)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendVolumeEvent(volume, @event, actorId, platformId);
    }
}
