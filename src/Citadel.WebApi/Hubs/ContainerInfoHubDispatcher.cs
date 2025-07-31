using Application.Services.Abstractions;
using Domain.Contracts.Resources.Containers;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

internal sealed class ContainerInfoHubDispatcher(IHubContext<ContainerInfoHub, ITypedContainerInfoHub> hubContext) : IContainerInfoHubDispatcher
{
    public Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken)
    => hubContext.Clients.Group(ContainerInfoHub.GroupName(container.ContainerId)).ReceiveContainerInfo(container);
}