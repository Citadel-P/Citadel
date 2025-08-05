using Application.Services.Abstractions;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using static Hosting.Common.Constants;

namespace WebApi.Hubs.Dispatchers;

internal sealed class DockerDaemonHubDispatcher(IHubContext<DockerDaemonHub, ITypedDockerDaemonHub> hubContext) : IDockerDaemonHubDispatcher
{
    public Task SendContainerEvent(Container container, string @event)
        => hubContext.Clients.Group(SignalRGroups.DockerDaemonGroup(container.PlatformId)).ContainerEventReceived(ContainerView.Map(container), @event);
}