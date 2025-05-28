using Infrastructure.Entities;
using Infrastructure.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public Task PushPlatformUpdate(Platform platform)
        => hubContext.Clients.All.PlatformUpdated(platform.Map());

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms) 
        => hubContext.Clients.All.PlatformsUpdated(PlatformsView.Map(platforms).Platforms);
}