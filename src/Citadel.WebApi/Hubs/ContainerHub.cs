using Application.Features.Platforms.Queries;
using Application.Services.Abstractions;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

public interface ITypedContainerHub
{
    Task ContainerEventReceived(ContainerView message, string @event);
    Task ContainersInfoUpdated(ContainersView message);
    Task ContainersStatsUpdated(IEnumerable<ContainerStatView> containers);
}

[Authorize]
internal sealed class ContainerHub(IMediator mediator, ISignalRConnectionTracker connectionTracker) : HubBase<ITypedContainerHub>(connectionTracker)
{
    public async Task<ContainersView> GetContainers(Guid id)
    {
        var response = await mediator.Send(new GetContainers(id));
        if (response.IsSuccess(out var containers))
        {
            return ContainersView.Map(containers);
        }
        else return new ContainersView([]);
    }
}