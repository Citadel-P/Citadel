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
    public async Task SendPlatformUpdated(IEnumerable<PlatformView> platforms)
    {
        await Clients.All.PlatformsUpdated(platforms);
    }
}
