using Domain.Entities;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
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

    public Task PushPlatformStats(PlatformStatsBatch platform)
        => hubContext.Clients.Group("Platforms").PlatformStatsUpdated(PlatformStatsBatchView.Map(platform));
}