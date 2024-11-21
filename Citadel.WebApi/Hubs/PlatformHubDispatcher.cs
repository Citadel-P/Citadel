using Application.Services.Abstractions;
using Infrastructure.Entities;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public async Task SendPlatformUpdated(Platform platform) 
        => await hubContext.Clients.All.PlatformUpdated(platform);
}