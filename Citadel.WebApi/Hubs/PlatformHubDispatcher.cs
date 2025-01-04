using Application.Services.Abstractions;
using Infrastructure.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Hubs;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public async Task SendPlatformUpdated(Platform platform) 
        => await hubContext.Clients.All.PlatformUpdated(Mapper.Map(platform));
}