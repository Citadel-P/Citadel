using Application.Features.Platforms.Queries;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformsUpdated(IEnumerable<PlatformView> platforms);
    Task PlatformUpdated(PlatformView platform);
}

[Authorize]
internal sealed class PlatformHub(IMediator mediator) : Hub<ITypedPlatformHub>
{
    public Task SendPlatformUpdated(IEnumerable<PlatformView> platforms) =>
        Clients.All.PlatformsUpdated(platforms);

    public async Task<PlatformsView> GetPlatforms()
    {
        var result = await mediator.Send(new GetPlatforms());
        if (result.IsSuccess(out var platforms))
        {
            return PlatformsView.Map(platforms);
        }
        else return new PlatformsView([]);
    }
}
