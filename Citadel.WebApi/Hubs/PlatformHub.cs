using Infrastructure.Entities;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Controllers.V1.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformUpdated(PlatformView platform);
}

[Authorize]
internal sealed class PlatformHub : Hub<ITypedPlatformHub>
{
    public async Task SendPlatformUpdated(PlatformView platform)
    {
        await Clients.All.PlatformUpdated(platform);
    }
}
