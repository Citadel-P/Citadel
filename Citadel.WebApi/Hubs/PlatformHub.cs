using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformsUpdated(IEnumerable<PlatformView> platforms);
}

[Authorize]
internal sealed class PlatformHub : Hub<ITypedPlatformHub>
{
    public Task SendPlatformUpdated(IEnumerable<PlatformView> platforms) =>
        Clients.All.PlatformsUpdated(platforms);
}
