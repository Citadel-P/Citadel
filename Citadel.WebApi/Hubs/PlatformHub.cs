using Infrastructure.Entities;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformUpdated(Platform platform);
}

[Authorize]
internal sealed class PlatformHub : Hub<ITypedPlatformHub>
{
    public async Task SendPlatformUpdated(Platform platform)
    {
        await Clients.All.PlatformUpdated(platform);
    }
}
