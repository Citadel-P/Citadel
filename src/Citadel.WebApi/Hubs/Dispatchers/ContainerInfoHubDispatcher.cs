using Application.Services.Abstractions;
using Domain.Contracts.Resources.Containers;
using Microsoft.AspNetCore.SignalR;
using static Hosting.Common.Constants;

namespace WebApi.Hubs.Dispatchers;

internal sealed class ContainerInfoHubDispatcher(IHubContext<ContainerInfoHub, ITypedContainerInfoHub> hubContext) : IContainerInfoHubDispatcher
{
    public Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken)
    => hubContext.Clients.Group(SignalRGroups.ContainerInfoGroup(container.ContainerId)).ReceiveContainerInfo(container);
}
