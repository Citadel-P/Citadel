using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Application.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public Task PushPlatformUpdate(Platform platform)
        => hubContext.Clients.Group("Platforms").PlatformUpdated(platform.Map());

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms) 
        => hubContext.Clients.Group("Platforms").PlatformsUpdated(PlatformsView.Map(platforms).Platforms);

    public Task PlatformDeleted(Guid platformId)
        => hubContext.Clients.Group("Platforms").PlatformsDeleted(platformId);

    public Task PushPlatformStats(Guid platformId, PlatformStatsResult platform)
        => hubContext.Clients.Group("Platforms").PlatformStatsUpdated(PlatformStatsBatchView.Map(platformId, platform));
}