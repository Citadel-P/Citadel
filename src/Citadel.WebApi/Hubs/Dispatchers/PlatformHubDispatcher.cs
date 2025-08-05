using Application.Services.Abstractions;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;
using static Hosting.Common.Constants;

namespace WebApi.Hubs.Dispatchers;

internal sealed class PlatformHubDispatcher(IHubContext<PlatformHub, ITypedPlatformHub> hubContext) : IPlatformHubDispatcher
{
    public Task PushPlatformUpdate(Platform platform)
        => hubContext.Clients.Group(SignalRGroups.PlatformsGroup).PlatformUpdated(platform.Map());

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms) 
        => hubContext.Clients.Group(SignalRGroups.PlatformsGroup).PlatformsUpdated(PlatformsView.Map(platforms).Platforms);

    public Task PlatformDeleted(Guid platformId)
        => hubContext.Clients.Group(SignalRGroups.PlatformsGroup).PlatformsDeleted(platformId);

    public Task PushPlatformStats(Guid platformId, PlatformStatsResult platform)
        => hubContext.Clients.Group(SignalRGroups.PlatformsGroup).PlatformStatsUpdated(PlatformStatsBatchView.Map(platformId, platform));
}