using Infrastructure.Entities;
using Infrastructure.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Hubs;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public async Task PushPlatformsUpdates(IEnumerable<Platform> platforms) 
        => await hubContext.Clients.All.PlatformsUpdated(Mapper.Map(platforms));
}